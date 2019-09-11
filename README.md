# libsrt-rs

Rust binding of the reference implementation of SRT (Secure Reliable
Transport).

Reference implementation is available at
https://github.com/haivision/srt

# Requirements

* cmake (as build system)
* pkg-config (as build system)
* OpenSSL
* Pthreads

## For Linux:

Install cmake, pkg-config and openssl-devel (or similar name) package.

### Ubuntu

```
sudo apt-get update
sudo apt-get upgrade
sudo apt-get install pkg-config cmake libssl-dev build-essential
```

### CentOS

```
sudo yum update
sudo yum install pkgconfig openssl-devel cmake gcc gcc-c++ make automake
```

## For Mac (Darwin, iOS):

```
brew install cmake
brew install openssl
```
