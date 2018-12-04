#!/usr/bin/env python3

import pytest
from pathlib import Path
import os, sys
from shutil import copytree, rmtree
from getkey import getkey
import subprocess
import shlex

"""

pytest t/test.py

TODO:
    move:
        existing file in ~ is symlink:
            to file in repo: do nothing
            to another file: copy content of the file, remove link, create new link
    git
"""


test_root = Path(__file__).absolute().parent
os.chdir(test_root)

home_test=Path(test_root, 'home_test')
common = Path(test_root, 'common')
meta = Path(common, 'meta.txt')
backup = Path(test_root, 'backup')
tune = Path(test_root, 'tune')

def setup():
    for d in [home_test, common, backup]:
        if d.exists():
            rmtree(d)

    copytree(Path(test_root, 'home'), home_test)
    common.mkdir()

confine_exe = Path(test_root, '../target/debug/confine').absolute()

def confine(*args):
    args = [ shlex.quote(str(arg)) for arg in args ]
    args = [str(confine_exe), '--trace', '--home', str(home_test), *args]
    print(' '.join(args))
    subprocess.run(args, check=True)

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
    hf = Path(home_test, '.test_conf')
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
    hf = Path(home_test, '.test_dir')
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


def test_ln():
    setup()
    confine('mv', 'common', '.test_conf')
    Path(home_test, '.test_conf').unlink()

    confine('ln', 'common/.test_conf')

    cf = Path(common, '.test_conf')
    hf = Path(home_test, '.test_conf')
    assert cf.exists()
    assert hf.exists()
    assert hf.is_symlink()
    assert hf.resolve() == cf

def test_ln_all_group():
    setup()
    confine('mv', 'common', '.test_conf', '.config/test_file', '.config/test_dir')
    Path(home_test, '.test_conf').unlink()
    Path(home_test, '.config/test_file').unlink()
    Path(home_test, '.config/test_dir').unlink()

    confine('ln', 'common')

    m = get_meta()

    for f in m:
        cf = Path(common, f)
        hf = Path(home_test, f)
        assert cf.exists()
        assert hf.exists()
        assert hf.is_symlink()
        assert hf.resolve() == cf

def test_ln_not_in_meta():
    setup()
    confine('mv', 'common', '.test_conf', '.config/test_file', '.config/test_dir')
    Path(home_test, '.test_conf').unlink()
    Path(home_test, '.config/test_file').unlink()
    Path(home_test, '.config/test_dir').unlink()

    with pytest.raises(subprocess.CalledProcessError):
        confine('ln', 'common', '.config')


def test_ln_backup():
    setup()
    
    confine('mv', 'common', '.test_conf')

    # ok, should skip existing link
    confine('ln', 'common', '.test_conf')

    test_conf = Path(home_test, '.test_conf')
    test_conf.unlink()
    test_conf.write_text('oh shi')


    confine('ln', 'common', '.test_conf')

    assert backup.exists()
    host = next(backup.iterdir())

    with open(Path(host, '.test_conf')) as f:
        t = f.read()
        assert t == 'oh shi'



def test_ln_backup_dir():
    setup()

    test_dir = Path(home_test, '.config/test_dir')

    confine('mv', 'common', '.config/test_dir')
    assert test_dir.is_symlink()

    confine('ln', 'common', '.config/test_dir')
    assert test_dir.is_symlink()

    test_dir.unlink()

    copytree(Path(common, '.config/test_dir'), Path(home_test, '.config/test_dir'))

    confine('ln', 'common', '.config/test_dir')
    assert test_dir.is_symlink()
    
    host = next(backup.iterdir())
    
    f = Path(host, '.config/test_dir/test_file')

    assert f.exists()


####### templates

def test_templates():
    setup()

    confine('mv', 'common', '.gitconfig')

    with pytest.raises(Exception):
        # is template
        confine('ln', 'common')

    with pytest.raises(Exception):
        # variable missing
        confine('ln', 'common', '-t', 'test')

    confine('ln', 'common', '-t', 'test2')
    confine('ln', 'common', '-t', 'test2.toml')
    confine('ln', 'common', '-t', 'tune/templates/test2.toml')

    gitconfig = Path(home_test, '.gitconfig')

    with open(gitconfig) as f:
        lines = [line.strip() for line in f]
        assert lines[0] == str(home_test)
        assert lines[1] == '1'

    # assert 0


