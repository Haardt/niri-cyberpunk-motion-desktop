#!/usr/bin/env python3
"""Real nested Wayland keyboard -> greetd IPC test. Uses only synthetic credentials."""
import json
import os
from pathlib import Path
import signal
import socket
import struct
import subprocess
import tempfile
import threading
import time

root = Path(__file__).resolve().parents[1]
runtime = Path(os.environ['XDG_RUNTIME_DIR'])
wtype = os.environ.get('WTYPE', str(root / '.test-deps/usr/bin/wtype'))

def receive(stream):
    def exact(size):
        value = b''
        while len(value) < size:
            part = stream.recv(size - len(value))
            assert part, 'unexpected IPC EOF'
            value += part
        return value
    return json.loads(exact(struct.unpack('=I', exact(4))[0]))

def send(stream, value):
    data = json.dumps(value).encode()
    stream.sendall(struct.pack('=I', len(data)) + data)

with tempfile.TemporaryDirectory(prefix='motion-input-') as temp:
    server = socket.socket(socket.AF_UNIX)
    server.bind(temp + '/greetd.sock')
    server.listen(1)
    server.settimeout(15)
    challenge = threading.Event()
    rejected = threading.Event()
    cancel_scan = threading.Event()
    cancelled = threading.Event()
    completed = threading.Event()
    errors = []
    def daemon():
        try:
            stream, _ = server.accept()
            with stream:
                stream.settimeout(15)
                request = receive(stream)
                assert request == {'type': 'create_session', 'username': 'tést'}, request
                send(stream, {'type': 'auth_message', 'auth_message_type': 'info', 'auth_message': 'Place your finger on the sensor'})
                assert receive(stream) == {'type': 'post_auth_message_response', 'response': None}
                time.sleep(0.3)
                send(stream, {'type': 'auth_message', 'auth_message_type': 'error', 'auth_message': 'Fingerprint timed out'})
                assert receive(stream) == {'type': 'post_auth_message_response', 'response': None}
                send(stream, {'type': 'auth_message', 'auth_message_type': 'secret', 'auth_message': 'Password:'})
                challenge.set()
                request = receive(stream)
                assert request == {'type': 'post_auth_message_response', 'response': 'test-pass'}, request
                send(stream, {'type': 'error', 'error_type': 'auth_error', 'description': 'Synthetic rejection'})
                assert receive(stream)['type'] == 'cancel_session'
                send(stream, {'type': 'success'})
                rejected.set()
                assert receive(stream) == {'type': 'create_session', 'username': 'canceluser'}
                send(stream, {'type': 'auth_message', 'auth_message_type': 'info', 'auth_message': 'Place your finger on the sensor'})
                assert receive(stream) == {'type': 'post_auth_message_response', 'response': None}
                cancel_scan.set()
                time.sleep(0.6)
                send(stream, {'type': 'success'})
                assert receive(stream)['type'] == 'cancel_session', 'late success started a cancelled session'
                send(stream, {'type': 'success'})
                cancelled.set()
                assert receive(stream) == {'type': 'create_session', 'username': 'testuser'}
                send(stream, {'type': 'auth_message', 'auth_message_type': 'info', 'auth_message': 'Place your finger on the sensor'})
                assert receive(stream) == {'type': 'post_auth_message_response', 'response': None}
                time.sleep(0.3)
                send(stream, {'type': 'success'})
                request = receive(stream)
                assert request['type'] == 'start_session', request
                assert request['cmd'] == ['niri-session'], request
                assert 'XDG_SESSION_TYPE=wayland' in request['env'], request
                assert 'XDG_CURRENT_DESKTOP=niri' in request['env'], request
                send(stream, {'type': 'success'})
                completed.set()
        except BaseException as error:
            errors.append(error)
    worker = threading.Thread(target=daemon, daemon=True)
    worker.start()
    before = set(runtime.glob('niri.*.sock'))
    render_before = set(runtime.glob('motion-login-*.sock'))
    with open('/tmp/motion-input.log', 'w') as log:
        process = subprocess.Popen(['niri', '-c', str(root / 'packaging/preview.kdl'), '--',
            str(root / 'target/release/motion-login'), '--config', str(root / 'login.toml'),
            '--background', str(root / 'background.toml')],
            env=dict(os.environ, GREETD_SOCK=temp + '/greetd.sock'), stdout=log, stderr=log, start_new_session=True)
        try:
            deadline = time.monotonic() + 10
            while time.monotonic() < deadline:
                nested = set(runtime.glob('niri.*.sock')) - before
                renders = set(runtime.glob('motion-login-*.sock')) - render_before
                if nested and renders:
                    break
                time.sleep(0.05)
            assert len(nested) == 1 and len(renders) == 1
            display = next(iter(nested)).name.split('.')[1]
            env = dict(os.environ, WAYLAND_DISPLAY=display)
            # Allow initial layer configure and keyboard focus to settle.
            time.sleep(1)
            subprocess.run([wtype, '-d', '30', 'tést🙂', '-k', 'BackSpace', '-k', 'Return'], env=env, check=True)
            assert challenge.wait(5), errors
            time.sleep(0.15)
            subprocess.run([wtype, '-d', '30', 'test-pass', '-k', 'Return'], env=env, check=True)
            assert rejected.wait(5), errors
            time.sleep(0.15)
            subprocess.run([wtype, '-d', '30', 'canceluser', '-k', 'Return'], env=env, check=True)
            assert cancel_scan.wait(5), errors
            subprocess.run([wtype, '-M', 'ctrl', '-k', 'u', '-m', 'ctrl'], env=env, check=True)
            assert cancelled.wait(5), errors
            time.sleep(0.15)
            subprocess.run([wtype, '-d', '30', 'testuser', '-k', 'Return'], env=env, check=True)
            assert completed.wait(5), errors
            worker.join(timeout=2)
            assert not errors, errors
            for _ in range(60):
                if not next(iter(renders)).exists():
                    break
                time.sleep(0.05)
            assert not next(iter(renders)).exists(), 'supervisor left its background running'
            print('Wayland fingerprint Info/Error handling, password fallback, Unicode input, in-flight cancellation, late-success rejection and fingerprint session handoff passed')
        finally:
            os.killpg(process.pid, signal.SIGTERM)
            process.wait(timeout=10)
            server.close()
