# megamonic
A badly designed multithreaded system monitor for 64bit linux for my personal use.

![Screenshot](screenshot.png)

## Build instructions for Arch
I think this is what is needed but I'm not sure.

If your ulimit -n is low you might have to increase it or you will get "Too many opened files" error.

Other than Rust-nightly you need these packages.  
Requirements: `glibc`, `gcc-libs` and`lm_sensors`  
Optional GPU support: `nvidia-utils`  

1. git clone https://github.com/meganomic/megamonic.git
2. cargo b --release

### FAQ

Q: Why only Nvidia GPU support?  
A: Because that's what I have

Q: Why another performance monitor?  
A: For fun

Q: Why multithreaded?  
A: For fun

Q: <*insert your question*>  
A: For fun
