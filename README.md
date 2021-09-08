# Rust Plugins: Third Trial

I had a [dylib demo](https://github.com/purpleposeidon/shiny-octo-memory), but it's broken lately with errors like:

```
error: cannot satisfy dependencies so `std` only shows up once
  |
  = help: having upstream crates all available in one format will likely make this go away
```

Setting `crate-type` seems to often cause these sorts of errors; it's fragile. Too bad we have to use it for plugins, hmm?

Another problem: the base crate gets all its symbols embedded into the dylibs, and they're rather bloated; even "hello world" takes a second to compile.
