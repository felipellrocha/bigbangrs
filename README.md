# bigbangrs

![bigbangrs](https://imgur.com/PvsHAyZ.gif)

A small (and very experimental) toy project where I tried to simulate
a “big bang”-style particle system using modern GPU compute.

This started as a 2-day sprint to learn how to use compute pipelines
and modern graphics APIs. I’ve spent many years working with web engines,
but had never really touched modern GPU compute directly, so this was my
excuse to dive in and figure it out.

## What it does

* Simulates **~3 million particles** in real time
* Maintains **interactive framerates** (depending on your hardware)
* Uses a **voxel-based gravity field** that particles both contribute to and are influenced by
* Fully GPU-driven simulation (compute + rendering)

The core idea is:

* Particles deposit “mass” into a voxel grid
* The grid is blurred / accumulated over time
* Particles sample that field to derive a force (a kind of fake gravity)
* That force drives the motion of the system

The result is something that *looks* like emergent structure forming out of chaos—which is exactly what I was going for.

## What I’m proud of

This was built very quickly, and I’m genuinely happy with how far it got in a short amount of time:

* I implemented the **compute pipelines, rendering, and core simulation logic myself**
* Learned how to structure GPU data flows (buffers, bind groups, dispatch, etc.)
* Got a reasonably large particle count running interactively
* Built a full feedback loop between particles and a volumetric field
* The gravity field can be visualized directly by rendering the voxel grid (not implemented)

For a first serious pass at modern GPU compute, I’ll take it.

## Performance notes

One thing that surprised me: I tapped out at a relatively low number of particles.

I’ve pushed GPUs much harder in the past on the web alone (for example, rendering tens of millions of grass blades in a game), so I expected to be able to go further here. That didn’t quite happen.

I’m not entirely sure where the bottleneck is:

* WebGPU vs Metal vs wgpu overhead
* Something in my pipeline setup
* Or possibly using a feature that’s silently falling back to software

I’m currently running this on an **M1**, so I’d expect significantly better performance on something like an RTX 30xx or newer on Windows.

This project appears to be almost entirely **GPU-bound**, but I wouldn’t be shocked if I’m wrong there. If you dig into it and find something obvious, I’d love to know.

## What it is *not*

Let’s be clear—this is not physically accurate.

* No elastic or inelastic collisions
* Gravity is **not real gravity** (it’s a sampled scalar field with some heuristics)
* No conservation laws are enforced
* I am not a physicist, and it shows

This is much closer to a **visual / emergent system** than a scientific simulation.

## Why I made this

Mostly curiosity.

I wanted to:

* Understand compute shaders beyond surface-level usage
* See how far I could push a GPU-driven particle system quickly
* Build intuition for working with large datasets on the GPU

Also, it’s just fun to watch millions of particles swirl around.

## A note on AI usage

I used AI tools to help scaffold parts of the project (including this README).
The core rendering, compute setup, and simulation logic were written by me.

## Thanks

Huge thanks to the **Rust** and **wgpu** teams for the incredible work they’ve done. Being able to even attempt something like this with relatively little setup is pretty amazing.

## Contributing

If you find this interesting and want to improve it—please do.

* Open a PR
* I’ll review it
* If it looks good, I’ll merge it

There’s a lot of room to take this further (collisions, better force models, stability improvements, etc.).

## Running it

- Install Rust
- Call `cargo run`

## Final thoughts

This is rough, probably wrong in a lot of ways, and definitely not production-quality—but it works, it runs fast, and it taught me a lot.

That’s a win.

