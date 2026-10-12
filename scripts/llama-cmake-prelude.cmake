# Keep llama.cpp's vendored cpp-httplib off OpenSSL (#243).
#
# WHY THIS FILE EXISTS. `llama-cpp-sys-2` builds llama.cpp's `install` target,
# which includes `vendor/cpp-httplib` even though `LLAMA_BUILD_SERVER` is already
# OFF. That vendored copy predates OpenSSL 3.5, where `X509_get_subject_name`
# started returning `const X509_NAME *`, so it no longer compiles against the
# OpenSSL this machine has:
#
#   httplib.cpp:12574: cannot initialize a variable of type 'X509_NAME *'
#                      with an rvalue of type 'const X509_NAME *'
#
# sensei embeds llama.cpp for INFERENCE. It never serves HTTP from it and never
# downloads over it — `LLAMA_CURL` is OFF too — so the TLS half of a vendored
# HTTP library is a component we pay for and never call.
#
# A TOOLCHAIN FILE, which is not where this obviously belongs, because it is the
# only hook that runs EARLY ENOUGH. `llama-cpp-sys-2`'s build.rs forwards only
# environment variables whose names start with `CMAKE_`, so `-DLLAMA_OPENSSL=OFF`
# cannot be passed directly; and `option()` will not override a cache entry that
# already exists, so the entry has to be in place before llama.cpp's own
# CMakeLists is read. A toolchain file is read before `project()`.
#
# Setting ONE cache variable and nothing else: CMake still detects the compiler,
# the SDK and the target the way it would with no toolchain file at all.
set(LLAMA_OPENSSL OFF CACHE BOOL "sensei: cpp-httplib needs no TLS here (#243)" FORCE)
