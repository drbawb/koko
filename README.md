koko (ここ)
===========

sometimes you just need to jot down an idea

you don't want to think about resolutions or DPI...  
or about resizing your canvas...  
or panning your image about...  

you just want a blackboard that goes on forever

koko is here: the simple, infinite canvas.

---

## disclaimer

this is software is still under initial development.   
at the time of writing: koko can't even save or restore images.

assume nothing works and that everything is broken.

## building koko

Koko is written in `rust`, you will need the following:

- a working rust toolchain w/ `rustc` and `cargo`
- GPU & Drivers supporting OpenGL 3.1 and newer

Once you have that, just clone this project and run:

- `cargo build` to build the executable or...
- `cargo run` to run it

## todo

* [ ] load/store support
* [ ] consider sparse data-structures to help w/ unused regions, etc.
* [X] dynamically reallocate regions
* [X] support drawing across region boundaries
* [X] basic brushes, colors, etc.
* [ ] everything else ...




