#! /bin/sh

FILTER_LINUX="" exec ./toolchain/lld-link /LIBPATH:"./wdk/Program Files/Windows Kits/10/Lib/10.0.15063.0/um/x64/" /LIBPATH:"./wdk/Program Files/Microsoft Visual Studio 14.0/VC/lib/amd64/" /LIBPATH:"./wdk/Program Files/Windows Kits/10/Lib/10.0.15063.0/ucrt/x64/" $@

link.exe %*

@REM Yes, this is cursed. .cargo/config lets you specify per-target, but not per-host.
@REM Let us be gladdened that there's basically nothing to do in here.
