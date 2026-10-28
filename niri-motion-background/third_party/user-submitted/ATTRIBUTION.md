# User-submitted shader ports

These shader assets are separate from the Rust implementation's MIT license. Original author/license notices remain attached to the source. They were added at the user's request for this local wallpaper installation; no public release or relicensing is implied.

| Local entry | Attribution / provenance | Port changes |
| --- | --- | --- |
| inercia-royale-2022 | User-supplied Inercia Shader Royale 2022 third-place entry; author/license not included in the snippet | Four deterministic lens samples, coherent roughness, continuous time/style, smooth curve interpolation, steady camera/lens and safe normals. |
| sky-path | Nusan, SkyPath, https://lezanu.fr/LiveCode/SkyPath.glsl , user-supplied source | Constant-speed camera glide, fixed focal length, gentle banking and soft highlight rolloff. |
| corroded-beasts | Nusan, original Bonzomatic source at https://lezanu.fr/LiveCode/CorrodedBeasts.glsl | OpenGL 3.3, time/resolution binding, unused FFT/sampler declarations removed. No audio. |
| fractal-cartoon | Kali, Fractal Cartoon / DE edge detection, https://www.shadertoy.com/view/XsBXWt ; portable source from Kodi visualization.shadertoy | Silent sun sizing; missing Nyan image replaced by original generated pixel sprite; shader input aliases. |
| cube-lines | Danil (2021+), https://www.shadertoy.com/view/NslGRN , CC BY-NC-SA 3.0 | Uses the original author's array-free GNOME port from https://github.com/danilw/GLSL-howto/tree/master/gnome_gjs/extensions/cubelines%40morimea.shadertoy.com ; fixed wallpaper presets, opaque output, no external input. |
| synthwave-sunset | stduhpf, another synthwave sunset thing, https://www.shadertoy.com/view/tsScRK | No audio, no AA/motion-blur jitter, 160 bounded tracing steps, safe denominators. |
| mandelbulb | evilryu, CC BY-NC-SA 3.0 | Initialized miss result; safe logarithm/division; no audio; opaque alpha. |
| seascape-tdm | Alexander Alekseev / TDM (2014), https://www.shadertoy.com/view/Ms2SD1 , CC BY-NC-SA 3.0 | OpenGL 3.3, fixed mouse, bounded normal epsilon and safe color powers. |
| cloud-orbit | User-supplied compact volumetric shader; no author/license supplied | Opaque output and low-FPS/low-resolution profile. |
| cloud-sunlit | User-supplied volumetric shader; no author/license supplied | Initialized the cross-product temporary and stable pixel jitter seed; safe bound square root; opaque alpha. |
| neon-menger | User-supplied reflective fractal; no author/license supplied | Safe normal/color evaluation, OpenGL wrapper, capped GPU budget. |
| blue-rectangles | User-supplied blue field and moving rectangles; no author/license supplied | Safe divisions/blur width; OpenGL wrapper. |

CC BY-NC-SA 3.0: https://creativecommons.org/licenses/by-nc-sa/3.0/ . Assets carrying this notice retain those terms and are not MIT-licensed. For the entries without a stated license, no extra rights are asserted.

`generated-cat.glsl` is an original procedural pixel-sprite replacement, drawn from rectangles and a palette rather than copied from an external texture image. It introduces no image download, audio texture, audio capture or sound playback. The shader renders the generated sprite directly on the GPU.

Skipped at the user's explicit request: Neon Ricochet's final postprocessing pass and Inigo Quilez's rainforest final postprocessing pass. The snippets have no scene-generation buffers, so they cannot reproduce the intended scenes by themselves. Neither appears as a misleading black/placeholder wallpaper entry.

Wallpaper tuning: the Fractal Cartoon port fixes the initial field of view and slows the sun/rainbow animation. Seascape uses a steady horizon and very subtle camera sway instead of a continuous camera rotation. Neon Menger uses a smooth shape parameter instead of the cusp in `pow(abs(sin(...)), .2)`. All per-entry animation speeds are reduced in config. The original source snapshots remain available for comparison.

Pixel-stability update: Inercia uses smooth material roughness, bounded emissive intensity and central-difference normals. The SkyPath/Corroded wallpaper ports use a fixed ray-step factor rather than screen-space random stepping. Optional GPU spatial/temporal stabilization is provided by the Rust renderer, separately from these source assets.
