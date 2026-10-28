#!/usr/bin/env python3
"""Real nested Wayland test; uses no greetd/PAM and preserves desktop service."""
import json
import os
from pathlib import Path
import signal
import socket
import subprocess
import time
import tempfile

root = Path(__file__).resolve().parents[1]
runtime = Path(os.environ['XDG_RUNTIME_DIR'])
before = set(runtime.glob('niri.*.sock'))
temporary = None
preview_args = [str(root / 'preview.sh')]
if os.environ.get('MOTION_TEST_GLITCH'):
    temporary = tempfile.NamedTemporaryFile(mode='w', suffix='.toml')
    temporary.write((root / 'login.toml').read_text().replace('intervalSeconds = 10', 'intervalSeconds = 5').replace('durationMs = 350', 'durationMs = 500'))
    temporary.flush()
    preview_args.append(temporary.name)
with open('/tmp/motion-login-preview.log', 'w') as log:
    process = subprocess.Popen(preview_args, stdout=log, stderr=log, start_new_session=True)
    try:
        deadline = time.monotonic() + 15
        state = None
        while time.monotonic() < deadline:
            assert process.poll() is None, 'nested compositor exited'
            for path in runtime.glob('motion-login-*.sock'):
                try:
                    with socket.socket(socket.AF_UNIX) as client:
                        client.settimeout(2)
                        client.connect(str(path))
                        client.sendall(b'status\n')
                        chunks = []
                        while part := client.recv(8192):
                            chunks.append(part)
                    state = json.loads(b''.join(chunks))
                    if state['outputs'] and state['outputs'][0]['frames'] > 2:
                        break
                except (OSError, ValueError):
                    continue
            if state and state['outputs'] and state['outputs'][0]['frames'] > 2:
                break
            time.sleep(0.2)
        if temporary:
            # Wait through a complete real GPU burst and restoration; the UI must stay alive.
            time.sleep(6)
        assert state and state['outputs'][0]['frames'] > 2, 'no GPU frames'
        assert state['wallpaper'] == 'cyberpunk-city-rain'
        assert state['fps'] <= 24
        nested = set(runtime.glob('niri.*.sock')) - before
        assert len(nested) == 1
        env = dict(os.environ, NIRI_SOCKET=str(next(iter(nested))))
        screenshot = Path('/tmp/motion-login-preview.png')
        if screenshot.exists():
            screenshot.unlink()
        subprocess.run(['niri', 'msg', 'action', 'screenshot-screen', '--path', str(screenshot)], env=env, check=True)
        for _ in range(40):
            if screenshot.exists():
                break
            time.sleep(0.1)
        assert screenshot.exists(), 'screenshot missing'
        supervisor = int(path.stem.rsplit('-', 1)[1])
        children = [int(pid) for pid in Path(f'/proc/{supervisor}/task/{supervisor}/children').read_text().split()]
        frontend = next(pid for pid in children if b'--frontend-worker' in Path(f'/proc/{pid}/cmdline').read_bytes())
        os.kill(frontend, signal.SIGTERM)
        for _ in range(60):
            if not path.exists() and all(not Path(f'/proc/{pid}').exists() for pid in children):
                break
            time.sleep(0.05)
        assert not path.exists(), 'background socket was not cleaned up'
        assert all(not Path(f'/proc/{pid}').exists() for pid in children), 'orphan child process'
        print('Frontend termination cleaned up both greeter workers')
        print(json.dumps(state, indent=2))
        print(f'Nested preview rendered; screenshot: {screenshot}')
    finally:
        os.killpg(process.pid, signal.SIGTERM)
        process.wait(timeout=10)
