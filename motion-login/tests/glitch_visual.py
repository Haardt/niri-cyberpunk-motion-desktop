#!/usr/bin/env python3
"""Capture before/during/after the configured three-second startup GPU burst."""
import os
import signal
import subprocess
import time
from pathlib import Path
from PIL import Image, ImageChops

root = Path(__file__).resolve().parents[1]
runtime = Path(os.environ['XDG_RUNTIME_DIR'])
before = set(runtime.glob('niri.*.sock'))
with open('/tmp/motion-glitch-visual.log', 'w') as log:
    p = subprocess.Popen([str(root / 'preview.sh')], stdout=log, stderr=log, start_new_session=True)
    try:
        deadline = time.monotonic() + 10
        while time.monotonic() < deadline:
            nested = set(runtime.glob('niri.*.sock')) - before
            sockets = list(runtime.glob('motion-login-*.sock'))
            if nested and sockets:
                parent = int(sockets[0].stem.rsplit('-', 1)[1])
                children = Path(f'/proc/{parent}/task/{parent}/children').read_text().split()
                frontends = [pid for pid in children if b'--frontend-worker' in Path(f'/proc/{pid}/cmdline').read_bytes()]
                if frontends:
                    break
            time.sleep(0.02)
        assert frontends, 'frontend did not start'
        start = int(Path(f'/proc/{frontends[0]}/stat').read_text().split()[21]) / os.sysconf('SC_CLK_TCK')
        env = dict(os.environ, NIRI_SOCKET=str(next(iter(nested))))
        images = []
        for name, offset in [('before', 2.7), ('during', 3.09), ('after', 4.0)]:
            delay = start + offset - time.monotonic()
            if delay > 0:
                time.sleep(delay)
            image = Path(f'/tmp/motion-glitch-{name}.png')
            image.unlink(missing_ok=True)
            subprocess.run(['niri', 'msg', 'action', 'screenshot-screen', '--path', str(image)], env=env, check=True)
            images.append(image)
        # niri encodes PNGs asynchronously. Do not wait for one PNG before
        # requesting the next timed frame, or the short effect will be missed.
        for image in images:
            for _ in range(500):
                try:
                    with Image.open(image) as frame:
                        frame.load()
                    break
                except (OSError, ValueError):
                    time.sleep(0.01)
        images = [Image.open(image).convert('RGB') for image in images]
        w, h = images[0].size
        x, y = (w - 1080) // 2, (h - 783) // 2
        # Opaque interior: changing wallpaper outside the panel cannot affect comparison.
        box = (x + 60, y + 240, x + 1010, y + 620)
        crops = [im.crop(box) for im in images]
        changed = [sum(any(c) for c in ImageChops.difference(crops[0], im).get_flattened_data()) for im in crops[1:]]
        assert changed[0] > 20, f'no visible GPU distortion: {changed}'
        assert changed[1] == 0, f'panel did not return to the original image: {changed}'
        print(f'GPU glitch changed {changed[0]} panel pixels; after burst: {changed[1]}')
    finally:
        os.killpg(p.pid, signal.SIGTERM)
        p.wait(timeout=10)
