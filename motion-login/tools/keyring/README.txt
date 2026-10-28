Optional TPM-backed keyring unlock for motion-login

Run from the motion-login source directory in your own terminal:

  sudo python3 tools/keyring/setup.py held

Enter sudo's password and then the CURRENT login keyring password locally.
Do not send passwords through chat. The installer stores only a systemd-creds
host+tpm2 encrypted credential, sealed to PCR 7, in a root-only directory.
The root-owned PAM helper decrypts it after successful motion-login
authentication and drops privileges before handing it through stdin to
gnome-keyring-daemon. No plaintext password is written to disk or argv.
Python/subprocess necessarily hold the decrypted password briefly in memory.

This is an optional convenience mechanism, not native biometric encryption.
The TPM does not check a fingerprint: PAM gates execution of the helper.
Root can recover the stored secret on this machine. Protecting the root
filesystem and boot chain still matters; this does not resist a compromised OS.
Any successful motion-login authentication method enables this unlock.
The user's other keyrings are not modified. Their passwords may differ.

The helper is optional: TPM failures leave the normal unlock dialog available.
Secure Boot policy/firmware changes, TPM reset, or a keyring password change may
require enrollment again. The installer exercises the helper but its exit code
does not prove the supplied password unlocked the keyring. Check at next login.
Real TPM enrollment and fingerprint login have to be tested on the machine.

Rollback: remove the session line referring to
/usr/local/libexec/motion-login-keyring from /etc/pam.d/motion-login.
Then remove /etc/motion-login/keyring/1000.cred to stop retaining the secret.
The installer keeps /etc/pam.d/motion-login.before-tpm-keyring as a backup.
Package PAM upgrades may require retaining the added session line manually.
