#!/usr/bin/python3
"""Root-owned pam_exec session helper. Never log credential contents."""
import os
import pwd
import subprocess
from pathlib import Path


def main():
    if (os.geteuid() != 0 or os.environ.get('PAM_TYPE') != 'open_session'
            or os.environ.get('PAM_SERVICE') != 'motion-login'):
        return
    user = pwd.getpwnam(os.environ['PAM_USER'])
    if user.pw_uid < 1000 or user.pw_uid == 65534:
        return
    credential = Path(f'/etc/motion-login/keyring/{user.pw_uid}.cred')
    if not credential.is_file():
        return
    result = subprocess.run(
        ['/usr/bin/systemd-creds', 'decrypt', '--name=motion-login-keyring',
         '--refuse-null', '--no-ask-password', str(credential), '-'],
        check=True, stdout=subprocess.PIPE, stderr=subprocess.DEVNULL, timeout=20,
        env={'PATH': '/usr/bin:/bin'})
    runtime = f'/run/user/{user.pw_uid}'
    # Drop supplementary groups and privileges before contacting the user's daemon.
    subprocess.run(
        ['/usr/bin/setpriv', f'--reuid={user.pw_uid}', f'--regid={user.pw_gid}',
         '--init-groups', '--no-new-privs', '/usr/bin/gnome-keyring-daemon', '--unlock'],
        input=result.stdout, check=True, timeout=15,
        stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL,
        env={'PATH': '/usr/bin:/bin', 'HOME': user.pw_dir, 'USER': user.pw_name,
             'LOGNAME': user.pw_name, 'XDG_RUNTIME_DIR': runtime,
             'DBUS_SESSION_BUS_ADDRESS': f'unix:path={runtime}/bus'})


if __name__ == '__main__':
    try:
        main()
    except Exception:
        # Optional session integration must not prevent login; the normal keyring
        # dialog remains available after TPM policy changes or other failures.
        raise SystemExit(1)
