#!/usr/bin/env python3
"""Live Wayland integration checks. Temporarily draws wallpapers; no config installs."""
import argparse
import json
import os
from pathlib import Path
import struct
import subprocess
import tempfile
import time
import tomllib
import zlib

ROOT = Path(__file__).resolve().parents[1]
parser = argparse.ArgumentParser(description=__doc__)
parser.add_argument('--binary', type=Path, default=ROOT / 'target/release/niri-motion-background')
parser.add_argument('--video', type=Path)
args = parser.parse_args()
binary = str(args.binary.resolve())
socket = Path(os.environ['XDG_RUNTIME_DIR']) / os.environ.get('MOTION_RENDER_SOCKET', 'niri-motion-background.sock')
if socket.exists():
    raise SystemExit('Refusing to interrupt a running daemon; stop it before this live test.')


def ctl(*command, ok=True):
    p = subprocess.run([binary, *command], capture_output=True, text=True, timeout=8)
    assert (p.returncode == 0) == ok, (command, p.stdout, p.stderr)
    return json.loads(p.stdout)


def frames():
    return ctl('status')['outputs'][0]['frames']


def ticks(pid):
    values = Path(f'/proc/{pid}/stat').read_text().rsplit(')', 1)[1].split()
    return int(values[11]) + int(values[12])


def chunk(kind, data):
    return struct.pack('!I', len(data)) + kind + data + struct.pack('!I', zlib.crc32(kind + data))


with tempfile.TemporaryDirectory(prefix='nmb-smoke-') as tmp:
    tmp = Path(tmp)
    image = tmp / 'still.png'
    image.write_bytes(b'\x89PNG\r\n\x1a\n' + chunk(b'IHDR', struct.pack('!2I5B', 2, 2, 8, 2, 0, 0, 0)) + chunk(b'IDAT', zlib.compress(b'\x00\xff\x00\x00\x00\xff\x00' * 2)) + chunk(b'IEND', b''))
    bad_shader = tmp / 'bad.frag'
    bad_shader.write_text('this is not valid GLSL')
    config = tmp / 'config.toml'
    text = (ROOT / 'config.toml').read_text().replace('source = "shaders/', f'source = "{ROOT}/shaders/')
    text = text.replace('pause_on_idle = true', 'pause_on_idle = false').replace('pause_on_fullscreen = true', 'pause_on_fullscreen = false')
    text += f'\n[[wallpapers]]\nname="still"\ntype="image"\nsource={json.dumps(str(image))}\n'
    text += f'\n[[wallpapers]]\nname="broken"\ntype="shader"\nsource={json.dumps(str(bad_shader))}\n'
    if args.video:
        text += f'\n[[wallpapers]]\nname="video"\ntype="video"\nsource={json.dumps(str(args.video.resolve()))}\n'
    text = text.replace('[rotation]\nenabled = true', '[rotation]\nenabled = false').replace('[day_night]\nenabled = true', '[day_night]\nenabled = false')
    config.write_text(text)
    with (tmp / 'daemon.log').open('w+') as log:
        p = subprocess.Popen([binary, '-c', str(config), 'run'], stdout=log, stderr=log)
        try:
            for _ in range(100):
                if p.poll() is not None:
                    raise AssertionError(f'daemon exited {p.returncode}')
                if socket.exists():
                    status = ctl('status')
                    if status['outputs'] and status['outputs'][0]['frames']:
                        break
                time.sleep(.05)
            else:
                raise AssertionError('no configured output/frame')
            duplicate = subprocess.run([binary, '-c', str(config), 'run'], capture_output=True, timeout=5)
            assert duplicate.returncode != 0, 'duplicate daemon was accepted'
            assert 'another' in duplicate.stderr.decode()
            shader_names = [w['name'] for w in tomllib.loads(text)['wallpapers'] if w['type']=='shader' and w.get('enabled', True) and w['name']!='broken']
            for name in shader_names:
                ctl('set', name)
                for _ in range(50):
                    if frames() > 0:
                        break
                    time.sleep(.04)
                assert ctl('status')['wallpaper'] == name
                assert frames() > 0, f'{name}: no frame'
            print(f'PASS all {len(shader_names)} shaders render; duplicate instance rejected')
            rate = ctl('status')['fps']
            before = frames()
            start = time.monotonic()
            time.sleep(1.2)
            count = frames() - before
            elapsed = time.monotonic() - start
            assert 2 <= count <= rate * elapsed + 2, (count, rate, elapsed)
            print(f'PASS FPS ceiling: {count} frames / {elapsed:.2f}s at ceiling {rate}')
            ctl('pause')
            time.sleep(.15)
            before = frames()
            cpu_before = ticks(p.pid)
            time.sleep(.7)
            assert frames() == before, 'paused shader kept rendering'
            cpu_after = ticks(p.pid)
            assert cpu_after - cpu_before <= 3, 'paused process used substantial CPU'
            print(f'PASS paused renderer quiescent ({cpu_after-cpu_before} CPU ticks)')
            ctl('resume')
            time.sleep(.25)
            assert frames() > before
            ctl('set', 'topo')
            old = ctl('status')['wallpaper']
            ctl('set', 'broken', ok=False)
            assert ctl('status')['wallpaper'] == old
            config.write_text('fps = "invalid"')
            ctl('reload', ok=False)
            assert ctl('status')['wallpaper'] == old
            config.write_text(text)
            ctl('reload')
            print('PASS invalid shader/config rollback; reload')
            ctl('set', 'still')
            time.sleep(.3)
            before = frames()
            assert before > 0
            time.sleep(.7)
            assert frames() == before, 'static image redrew periodically'
            print('PASS static image has no redraw loop')
            if args.video:
                ctl('set', 'video')
                time.sleep(1.5)
                status = ctl('status')['outputs'][0]
                assert not status['video_failed'] and status['frames'] > 1, status
                assert status['decoder'] not in (None, 'loading'), status
                print('PASS video playback; decoder:', status['decoder'])
                ctl('pause')
                time.sleep(.2)
                before = frames()
                time.sleep(.5)
                assert frames() == before
                ctl('resume')
                time.sleep(.4)
                assert frames() > before
                print('PASS video pause/resume')
            # Enable a short interval to exercise the actual event-loop scheduler.
            short = text.replace('interval_seconds = 600', 'interval_seconds = 1').replace('[rotation]\nenabled = false', '[rotation]\nenabled = true')
            config.write_text(short)
            ctl('reload')
            old = ctl('status')['wallpaper']
            time.sleep(1.25)
            assert ctl('status')['wallpaper'] != old, 'rotation did not advance'
            ctl('pause')
            old = ctl('status')['wallpaper']
            time.sleep(1.25)
            assert ctl('status')['wallpaper'] == old, 'rotation ran while paused'
            ctl('resume')
            time.sleep(.25)
            assert ctl('status')['wallpaper'] != old, 'overdue rotation did not resume'
            ctl('auto-switch', 'off')
            old = ctl('status')['wallpaper']
            time.sleep(1.25)
            assert ctl('status')['wallpaper'] == old
            print('PASS automatic rotation, deferral, resume and off switch')
            ctl('mode', 'night')
            assert ctl('status')['period'] == 'night'
            listing = ctl('list')['wallpapers']
            assert not next(w for w in listing if w['name']=='god-rays')['eligible']
            assert next(w for w in listing if w['name']=='black-hole')['eligible']
            for _ in range(3):
                ctl('next')
                assert ctl('status')['wallpaper'] not in ('god-rays','arrakis','my-video')
            ctl('mode', 'day')
            assert ctl('status')['period'] == 'day'
            listing = ctl('list')['wallpapers']
            assert not next(w for w in listing if w['name']=='black-hole')['eligible']
            assert next(w for w in listing if w['name']=='god-rays')['eligible']
            ctl('mode', 'auto')
            assert ctl('status')['period'] == 'any'
            ctl('prev')
            print('PASS day/night filtering and manual/automatic mode selection')
            ctl('quit')
            p.wait(timeout=8)
            assert p.returncode == 0
            assert not socket.exists(), 'socket was not cleaned up'
        except BaseException:
            log.flush()
            print((tmp / 'daemon.log').read_text())
            raise
        finally:
            if p.poll() is None:
                p.terminate()
                p.wait(timeout=8)
print('All live smoke tests passed.')
