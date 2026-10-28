#!/usr/bin/python3
"""Interactive root installer: password is read from the terminal, never argv."""
import getpass
import os
import pwd
import shutil
import subprocess
import sys
import tempfile
from pathlib import Path


def main():
    if os.geteuid() != 0 or len(sys.argv) != 2:
        sys.exit('Usage: sudo python3 tools/keyring/setup.py USERNAME')
    user = pwd.getpwnam(sys.argv[1])
    if user.pw_uid < 1000 or user.pw_uid == 65534:
        sys.exit('A regular desktop user is required.')
    pam = Path('/etc/pam.d/motion-login')
    original = pam.read_text()
    helper = Path('/usr/local/libexec/motion-login-keyring')
    line = f'session optional pam_exec.so quiet {helper}'
    secret = getpass.getpass('Keyring password (stays local): ')
    if not secret or secret != getpass.getpass('Repeat keyring password: '):
        sys.exit('Empty or mismatched passwords; nothing installed.')
    encrypted = subprocess.run(
        ['/usr/bin/systemd-creds', 'encrypt', '--with-key=host+tpm2',
         '--tpm2-pcrs=7', '--name=motion-login-keyring', '-', '-'],
        input=secret.encode(), stdout=subprocess.PIPE, check=True).stdout
    del secret
    directory = Path('/etc/motion-login/keyring')
    directory.mkdir(mode=0o700, exist_ok=True)
    os.chmod(directory, 0o700)
    credential = directory / f'{user.pw_uid}.cred'
    # Atomic replacement: an interrupted enrollment leaves the old secret intact.
    with tempfile.NamedTemporaryFile(dir=directory, delete=False) as target:
        target.write(encrypted)
        temporary = Path(target.name)
    temporary.replace(credential)
    helper.parent.mkdir(parents=True, exist_ok=True)
    shutil.copyfile(Path(__file__).with_name('unlock.py'), helper)
    os.chmod(helper, 0o755)
    env = {'PATH': '/usr/bin:/bin', 'PAM_TYPE': 'open_session',
           'PAM_SERVICE': 'motion-login', 'PAM_USER': user.pw_name}
    subprocess.run([str(helper)], env=env, check=True)
    backup = pam.with_name('motion-login.before-tpm-keyring')
    if not backup.exists():
        shutil.copy2(pam, backup)
    if line not in original.splitlines():
        # Must follow system-local-login, which creates the user runtime directory.
        with pam.open('a') as target:
            target.write('\n# Optional TPM-backed desktop keyring unlock\n' + line + '\n')
    print('Installed. Test at the next normal logout/login; no service restart needed.')


if __name__ == '__main__':
    main()
