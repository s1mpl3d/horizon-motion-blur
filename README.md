# Horizon Motion Blur

Cinematic, camera-based motion blur for [Horizon](https://github.com/horizon-rust/horizon), the Minecraft client rewritten in Rust.

The world is blurred along the camera's motion, like a real camera's shutter. Your hand, the HUD and menus are drawn after the blur, so they stay sharp.

## Features

- **Per-pixel blur.** Uses the world's depth to work out how far each pixel moved, rather than smearing whole frames together.
- **Hand and HUD stay sharp.** The blur runs before the first-person hand, the HUD and screens are drawn.
- **Consistent at any frame rate.** Optionally keeps the blur the same length as at 60 FPS, so 240 FPS doesn't make it disappear.
- **Adjustable.** Strength and sample count from the in-game settings page.
- **No teleport smears.** Frames where the camera jumps more than 16 blocks aren't blurred.
- **Zero cost when off.** Turn it off in Options > Mods and the pass is skipped.

## Installation

1. Download [`motion_blur.hnmod`](motion_blur.hnmod).
2. Put it in `%APPDATA%\horizon\horizon-native-mods\`.
3. Open the Horizon launcher and rebuild native mods.
4. In game, open **Options > Mods > Motion Blur**.

## Settings

| Setting | Values | Default |
| --- | --- | --- |
| Strength | 25%, 50%, 75%, 100%, 150%, 200% | 75% |
| Quality | 8, 12, 16, 24 samples | 12 |
| Same at any FPS | ON / OFF | ON |

With **Same at any FPS** off, the blur gets shorter as your frame rate rises, as with a physical camera.

Settings are stored in `%APPDATA%\horizon\horizon-mods\.horizon-mod-data\motion_blur\`.

## How it works

The mod hooks `native_world_drawn`, which runs once the world is drawn and before the hand and HUD. For each pixel it:

1. reads the world's depth (requested through `native_render_needs`);
2. rebuilds the point's position relative to the camera;
3. projects that position with last frame's camera matrices;
4. averages samples of the image along the on-screen difference between the two positions.

This is the same reprojection approach as [natural-motionblur](https://github.com/ItsPasi/natural-motionblur-fabric) and [motionblur](https://github.com/Noryea/motionblur-fabric) on Fabric.

## Compatibility

- Needs native hooks API 58 (`"hooks": 58`, the render hooks), so it builds on any Horizon that has them.
- Built against Horizon `fdc87aecdbd116bf1e08adcd062fcbaea8c2e2ca` (Minecraft 26.3).
- On OpenGL, if the frame's image can't be copied (`COPY_SRC`), the mod does nothing.

## Known limitations

- Only camera motion is blurred. Mobs and players moving in front of a still camera stay sharp.

Found a bug? Open an [issue](../../issues).

## Building from source

The launcher compiles the mod into the game, so there's no separate build step. To make the `.hnmod`, zip the contents of `motion_blur/` with `horizon-native.json` at the root of the zip.

| File | Purpose |
| --- | --- |
| `motion_blur/horizon-native.json` | Mod manifest |
| `motion_blur/mixins/motion_blur/horizon-mixin.toml` | Modules and hook injections |
| `motion_blur/mixins/motion_blur/runtime.rs` | Settings, WGSL shader and the blur pass |
| `motion_blur/mixins/motion_blur/*.rs` | One helper per hook |

## Credits

Inspired by [natural-motionblur](https://github.com/ItsPasi/natural-motionblur-fabric), [PolyBlur](https://github.com/Polyfrost/PolyBlur) and [motionblur](https://github.com/Noryea/motionblur-fabric).
