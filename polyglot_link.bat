#! /bin/sh

FILTER_LINUX="" exec ./clang/lld-link /LIBPATH:./msvc_x64 /LIBPATH:./msvc_vc_lib/ /LIBPATH:./msvc_ucrt

link.exe %*

@REM Yes, this is cursed. .cargo/config lets you specify per-target, but not per-host.
@REM Let us be gladdened that there's basically nothing to do in here.
@REM The empty batch scripts `#!.bat` and `FILTER_LINUX.bat` should keep this batch script from generating errors.
