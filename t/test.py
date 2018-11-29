#!/usr/bin/env python3

import pytest
from pathlib import Path
import os, sys
from shutil import copytree, rmtree
from getkey import getkey
import subprocess
import shlex

test_root = Path(__file__).absolute().parent
os.chdir(test_root)

test_home=Path(test_root, 'home_test')
common = Path(test_root, 'common')
meta = Path(common, 'meta.txt')

def setup():
    if test_home.exists():
        rmtree(test_home)
    if common.exists():
        rmtree(common)
    copytree(Path(test_root, 'home'), test_home)
    common.mkdir()

confine_exe = Path(test_root, '../target/debug/confine').absolute()

def confine(*args):
    args = [ shlex.quote(arg) for arg in args ]
    args = [confine_exe, '--home', str(test_home), *args]
    subprocess.run(args)

def get_meta(meta_file=None):
    meta_file = meta_file or meta
    with open(meta_file) as f:
        return set([ s.strip() for s in f ])

def test_add_file():
    setup()
    confine('mv', 'common', '.test_conf')
    m = get_meta()
    assert len(m) == 1
    assert '.test_conf' in m
    cf = Path(common, '.test_conf')
    hf = Path(test_home, '.test_conf')
    assert cf.exists()
    assert hf.exists()
    assert hf.is_symlink()
    assert hf.resolve() == cf

def test_add_dir():
    setup()
    # also test parse group/file
    confine('mv', 'common/.test_dir')
    m = get_meta()
    assert len(m) == 1
    cf = Path(common, '.test_dir')
    hf = Path(test_home, '.test_dir')
    assert cf.exists()
    assert hf.exists()
    assert hf.is_symlink()
    assert hf.resolve() == cf
    
def test_add_multiple():
    setup()
    confine('mv', 'common', '.test_dir', 'common/.test_conf')
    m = get_meta()
    assert m == {'.test_dir', '.test_conf'}


def test_add_nested_file():
    setup()
    confine('mv', 'common', '.config/test_file', '.config/test_dir')
    m = get_meta()
    assert m == {'.config/test_file', '.config/test_dir'}
