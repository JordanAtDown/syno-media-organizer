#!/usr/bin/env python3
"""Sign a Synology SPK file with a GPG detached signature (syno_signature.asc).

The signed content is the concatenation of SPK members in the order
expected by Synology Package Center:
  INFO, LICENSE, PACKAGE_ICON*.PNG (sorted), WIZARD_UIFILES/* (sorted),
  conf/* (sorted), package.tgz, scripts/* (sorted).

Usage:
  python3 scripts/sign-spk.py <spk_file>

The GPG key to use must already be imported in the current GPG keyring.
"""

import io
import re
import sys
import tarfile
import subprocess

ICON_RE = re.compile(r'^PACKAGE_ICON(?:_(?:120|256))?\.PNG$')
WIZARD_RE = re.compile(r'^WIZARD_UIFILES/.+$')
CONF_RE = re.compile(r'^conf/.+$')
SCRIPT_RE = re.compile(r'^scripts/.+$')


def collect_data(spk_path: str) -> bytes:
    data = bytearray()
    with tarfile.open(spk_path, 'r:') as spk:
        names = spk.getnames()
        sorted_names = sorted(names)

        def read(name: str) -> bytes:
            return spk.extractfile(name).read()

        if 'INFO' in names:
            data += read('INFO')
        if 'LICENSE' in names:
            data += read('LICENSE')
        for name in sorted_names:
            if ICON_RE.match(name):
                data += read(name)
        for name in sorted_names:
            if WIZARD_RE.match(name):
                data += read(name)
        for name in sorted_names:
            if CONF_RE.match(name):
                data += read(name)
        if 'package.tgz' in names:
            data += read('package.tgz')
        for name in sorted_names:
            if SCRIPT_RE.match(name):
                data += read(name)

    return bytes(data)


def sign_spk(spk_path: str) -> None:
    print(f"Collecting data to sign from {spk_path}…")
    data = collect_data(spk_path)

    print("Signing with GPG…")
    result = subprocess.run(
        ['gpg', '--batch', '--yes', '--detach-sign', '--armor', '--output', '-'],
        input=data,
        capture_output=True,
    )
    if result.returncode != 0:
        print(f"GPG error:\n{result.stderr.decode()}", file=sys.stderr)
        sys.exit(1)

    signature = result.stdout

    print(f"Adding syno_signature.asc to {spk_path}…")
    with tarfile.open(spk_path, 'a:') as spk:
        info = tarfile.TarInfo('syno_signature.asc')
        info.size = len(signature)
        spk.addfile(info, io.BytesIO(signature))

    print(f"Done — {spk_path} is now signed.")


if __name__ == '__main__':
    if len(sys.argv) < 2:
        print(f"Usage: {sys.argv[0]} <spk_file>", file=sys.stderr)
        sys.exit(1)
    sign_spk(sys.argv[1])
