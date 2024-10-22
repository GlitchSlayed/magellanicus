# Magellanicus

Magellanicus is a free Vulkan renderer that reimplements Halo: Combat Evolved's
graphics. It is written in Rust (and GLSL for shaders) and uses the [Vulkano]
framework, and it is licensed under version 3 of the GNU GPL.

[Vulkano]: https://github.com/vulkano-rs/vulkano

This renderer is heavily WIP right now and barely renders anything. Don't make
anything that uses it unless you want to see something that doesn't work and
also breaks regularly.

## Disclaimer

This project is NOT affiliated with the official developers of Halo. This is NOT
an official project. Support the official games!

## Table of contents

* [Minimum requirements]
* [How to run flycam-test (locally)]
* [TODO]
* [FAQ]
  * [Why make this?]
  * [Why Vulkan and not Direct3D, OpenGL, etc.?]
  * [Why no-std but also extern crate std?]
  * [Why don't Halo Custom Edition maps look exactly the same?]
  * [Why don't protected maps work?]
  * [Can I use MCC tags?]
* [Contributing]
* [License]

[Minimum requirements]: #minimum-requirements
[How to run flycam-test (locally)]: #how-to-run-flycam-test-locally
[FAQ]: #faq
[TODO]: #todo
[Why make this?]: #why-make-this
[Why Vulkan and not Direct3D, OpenGL, etc.?]: #why-vulkan-and-not-direct3d-opengl-etc
[Why no-std but also extern crate std?]: #why-no-std-but-also-extern-crate-std
[Why don't Halo Custom Edition maps look exactly the same?]: #why-dont-halo-custom-edition-maps-look-exactly-the-same
[Why don't protected maps work?]: #why-dont-protected-maps-work
[Can I use MCC tags?]: #can-i-use-mcc-tags
[Contributing]: #contributing
[License]: #license

## Minimum requirements

The basic requirements:
* Vulkan 1.3 compatible GPU, or...
* Vulkan 1.2 compatible GPU with `VK_EXT_extended_dynamic_state`; check this
  list if you are unsure: https://vulkan.gpuinfo.org/listdevices.php
* `VK_KHR_swapchain`, etc.
* Up-to-date drivers **(IMPORTANT: outdated drivers = closed issues!)**

These are a safe bet:
* **AMD:** AMD Radeon HD 7700 series or newer (GCN)
* **NVIDIA:** NVIDIA GeForce 600 series or newer (Kepler)
  * Note that the 600 series has some incompatible GPUs mixed in. However, the
    GTX 650 (Ti), GTX 660 (Ti), GTX 670, GTX 680, and GTX 690 should work.
* **Intel:** Intel HD Graphics 500 or newer (Skylake)

Tested GPUs:
* Nvidia GeForce RTX 4080 SUPER
* AMD Radeon RX 580
* AMD Radeon R9 280X (same as the AMD Radeon HD 7970)
* Apple M2

## How to run flycam-test (locally)

flycam-test is a tool for testing the renderer. It's a huge mess, but it works.

You need the following:
* Rust
* Vulkan ([MoltenVK] if on macOS)
* [SDL2] development libraries

[MoltenVK]: https://github.com/KhronosGroup/MoltenVK
[SDL2]: https://wiki.libsdl.org/SDL2/Installation

You do not necessarily need the Vulkan SDK to build this, but it does have some
nice tools to help with debugging.

```bash
git clone https://github.com/FishAndRips/magellanicus
cd magellanicus
cargo run --release
```

## TODO

These are what have to be done for the renderer to be complete.

* 🟡 Support all bitmap types
  * 🟢 2D textures
  * 🟢 Cubemaps
  * 🟢 3D textures
  * 🟡 Sprites
* 🟡 Support all 3D objects
  * 🟡 All shader groups with reasonable accuracy (accuracy can be improved later)
    * 🟡 `shader_environment`
      * Mostly finished; does not support ambient lighting yet.
    * 🔴 `shader_model`
      * Uses a fallback shader
    * 🟡 `shader_transparent_chicago` (+ `_extended`)
      * Mostly finished; some framebuffer operations not yet supported.
    * 🔴 `shader_transparent_generic`
      * Uses a fallback shader
    * 🔴 `shader_transparent_glass`
      * Uses a fallback shader
    * 🔴 `shader_transparent_meter`
      * Uses a fallback shader
    * 🔴 `shader_transparent_plasma`
      * Renders as white
    * 🔴 `shader_transparent_water`
      * Disabled
  * 🟡 Ambient fog
    * 🟢 Outdoor fog
    * 🔴 Indoor fog
  * 🔴 Ambient lighting
  * 🟡 BSPs
    * 🟡 Currently renders most geometry
    * 🔴 Fog planes
    * 🔴 Detail objects
  * 🔴 Shader animations
  * 🔴 Skyboxes
  * 🔴 Objects
  * 🔴 Shader inputs
  * 🔴 First-person models
  * 🔴 Animation interpolation
  * 🔴 Particles
  * 🔴 Particle systems
  * 🔴 Weather
  * 🔴 Decals
  * 🔴 Glow tags
  * 🔴 Lens flares
  * 🔴 Dynamic lighting
  * 🔴 Light volumes
* 🔴 HUDs
* 🔴 Menus
* 🔴 Text

## FAQ

### Why make this?

Our main goal is to make a replacement for the HEK. To do that, we need a Sapien
replacement (Sapien is the level editor) which requires 3D rendering.

There are a number of other use-cases, such as making a shader previewer in a
tag editor or potentially replacing the game's renderer.

### Why Vulkan and not Direct3D, OpenGL, etc.?

Vulkan is cross-platform and performant. Sure, it's verbose, but hey, it works!

OpenGL doesn't have very good drivers. Zink wraps OpenGL to Vulkan, and this
achieves [performance uplifts on some machines]. If native OpenGL is slower than
a wrapper, then wouldn't it be even faster to just target Vulkan directly?

[performance uplifts on some machines]: https://www.phoronix.com/news/Zink-2022-Refactor-Faster

On the other hand, Direct3D is proprietary and tied to Microsoft Windows. It may
be worth writing a D3D9 renderer in the future for old Windows XP PCs, but most
native D3D11/D3D12 cards can just use Vulkan directly.

### Why `#![no-std]` but also `extern crate std;`?

This may seem odd, but we want to leave this open to potentially more rendering
APIs in the future on systems that lack a standard library, so we're keeping our
options open. This decision may or may not change later, though.

Vulkano requires the Rust Standard Library, so the Vulkan renderer can make use
of it.

### Why don't Halo Custom Edition maps look exactly the same?

The Gearbox renderer infamously has a bunch of bugs. And Halo Custom Edition has
even more bugs. This renderer is not intended to replicate bugs.

### Why don't protected maps work?

Protected maps obfuscate the map so that it can't be read easily. The reason
Halo can still load these maps is that it's very context-sensitive. It doesn't
need the correct tag paths or tag groups set in a tag reference in a lot of
cases.

This renderer loads nearly everything upfront to maximize runtime performance,
and this means it needs to be able to read the map file and load everything
out-of-context.

### Can I use MCC tags?

Yes and no. We do not plan to support MCC's HUD system, but we do intend to
support most of its tags. Shader tags, BSPs, and models should function.

#### Is this project's license (the GPL) incompatible with MCC tags?

I am not a lawyer. However, the GPL says you can use this software for any
purpose, and that should include viewing MCC tags. The GPL isn't magic, though;
you still need permission to use those tags, and that isn't a condition of using
this software (thus not a GPL incompatibility) but a condition of using that
content.

## Contributing

We do accept pull requests, such as for fixing issues or improving accuracy.

## License

Magellanicus is licensed under version 3 of the GNU General Public License. It
is not licensed under any later or earlier version.
