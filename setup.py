from setuptools import setup, dist

dist.Distribution(dict(setup_requires=['rust-ext==0.1']))

from rust_ext import build_rust_cmdclass, install_lib_including_rust

setup(
    name='resbuild',
    version='0.1',
    description='esbuild rust extension',
    author='Joshua S. Miller',
    author_email='jsmiller@uchicago.edu',
    cmdclass={
        'build_rust': build_rust_cmdclass('Cargo.toml'),
        'install_lib': install_lib_including_rust
    },
    zip_safe=False,
)
