"""Generate the matching clear/rain shader variants from one scene source."""
from pathlib import Path
root = Path(__file__).resolve().parents[1]
source = (root / 'shaders/lib/cyberpunk-city.glsl').read_text()
for name, rain in [('cyberpunk-city-flight', 0), ('cyberpunk-city-rain', 1)]:
    (root / 'shaders' / (name + '.frag')).write_text(
        '#version 330 core\n#define CITY_RAIN ' + str(rain) + '\n' + source
    )
