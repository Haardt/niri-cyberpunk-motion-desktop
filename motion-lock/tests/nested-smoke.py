#!/usr/bin/env python3
"""Test the real lock protocol inside a separate Niri, never on the host socket."""
from pathlib import Path
import os, re, subprocess, tempfile, time

root = Path(__file__).resolve().parents[1]
binary = root / 'target/release/motion-lock'
with tempfile.TemporaryDirectory(prefix='motion-lock-nested-') as folder:
    tmp = Path(folder)
    log = open(tmp/'niri.log', 'w+')
    compositor = subprocess.Popen(['niri', '-c', str(root/'packaging/nested-test.kdl')], stdout=log, stderr=log)
    locker = None
    try:
        deadline = time.monotonic()+15
        socket = None
        while time.monotonic()<deadline:
            text=(tmp/'niri.log').read_text()
            match=re.search(r'IPC listening on: (\S+)', text)
            if match:
                socket=match[1];break
            if compositor.poll() is not None: raise RuntimeError(text)
            time.sleep(.1)
        assert socket, 'nested compositor did not start'
        display=re.search(r'niri\.(wayland-\d+)\.', socket)[1]
        env=dict(os.environ,WAYLAND_DISPLAY=display,NIRI_SOCKET=socket)
        broken=tmp/'broken.frag'
        broken.write_text('#version 330 core\nthis is not a shader;\n')
        invalid=subprocess.run([str(binary),'--lock','--pam-service','other','--shader',str(broken)],env=env,capture_output=True,text=True,timeout=10)
        assert invalid.returncode != 0 and 'preflight' in invalid.stderr, invalid.stderr
        authlog=open(tmp/'lock.log','w+')
        locker=subprocess.Popen([str(binary),'--lock','--pam-service','other'],env=env,stdout=authlog,stderr=authlog)
        deadline=time.monotonic()+15
        while 'session lock confirmed' not in (tmp/'lock.log').read_text():
            assert locker.poll() is None, (tmp/'lock.log').read_text()
            assert time.monotonic()<deadline, 'lock confirmation timed out'
            time.sleep(.1)
        second=subprocess.run([str(binary),'--lock','--pam-service','other'],env=env,capture_output=True,text=True,timeout=10)
        assert 'no unlock sent' in second.stderr, second.stderr
        locker.kill();locker.wait(timeout=5)
        time.sleep(.3)
        shot=tmp/'must-not-contain-desktop.png'
        subprocess.run(['niri','msg','action','screenshot-screen','--path',str(shot)],env=env,check=True,timeout=5,capture_output=True)
        assert not shot.exists(), 'nested desktop became capturable after locker crash'
        print('PASS: invalid shader refused before lock; lock acquired; second locker refused; capture remains blocked after crash')
    finally:
        if locker is not None and locker.poll() is None: locker.kill();locker.wait()
        compositor.terminate()
        try:compositor.wait(timeout=5)
        except subprocess.TimeoutExpired:compositor.kill();compositor.wait()
        log.close()
