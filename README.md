Confine - config manager
========================

USAGE
--------
Config files are divided into groups. When you move a file under confine's management, you assign
this file to a group.

First step. Move your current config files under `confine`'s control

```
# initialize config storage
[~]$ mkdir dotfiles && cd $_ && git init . && git commit -m 'init'

# create group common
[dotfiles]$ mkdir common

# move files into group common
# path could be absolute (but still belong to $HOME) or relative, assuming it's in ~
# this command will move files to common/ and create symlinks back to where the original files were
[dotfiles]$ confine move common .bashrc ~/.vimrc ~/.vim .config/git

# check
ls -lF ~/.bashrc
/home/user/.bashrc@ -> /home/user/dotfiles/common/.bashrc

```

Second step. Use your config files on another machine

```
[~]$ git clone ssh://.../git/dotfiles && cd dotfiles

# create links for all files in group common
confine link common

# create links only for some files (in group work)
confine link work .bashrc.work .config/my_app_setting
```

If there's already a file where link should be created, existing file is moved to `dotfiles/backup/$hostname/`

Options
-------

```
Common options:

    -q --quiet  -- be quiet
    -r --root <dir> -- config dir, default '.'

Subcommands:
    move | mv <group> [files]
    link | ln <group> [files]

```

Tips and tricks
---------------

Group and file name could be combined with `/`:
```
confine ln common/.bashrc # same as confine ln common .bashrc
```

Advanced usage
==============

Templates
---------

Some config files are not supporting includes or other types of customization.
For expample, it's hard to set ripgrep ignore file agnostic to user name:

```
cat $RIPGREP_CONFIG_PATH
--ignore-file = /home/nik/.config/ripgrep/ignore
```
This format does not support expanding of ~ or $HOME
So, I have to fix this on my other machines, where home dir is different.


Another expample is gitconfig. While there's still means to workaround this,
it would be so much easier, if you could just write
```
[user]
    name = {{GIT_NAME}}
    email = {{GIT_EMAIL}}
```

Sure, you could just create two groups: home and work, and create links from appropriate group.
But then, when you come up with great idea for config, you have to fix it in all files. Ain't nobody got time for this!

So, the solution is templates.

First, you create file under tune/templates:
```
cd ~/dotfiles && cat tune/templates/work.toml
[.gitconfig]
GIT_NAME = Nikita Bilous
GIT_EMAIL = nsbilous@example.com

```

What happens when you try to create link now? Let's find out:
```
confine ln common/.gitconfig
Error: common/.gitconfig : template required!

# skip files that require template
confine ln --skip common
Warning: common/.gitconfig template required, skipping

confine ln -t work common/.gitconfig
creating file tune/templates/common/.gitconfig

```

From now on, you either have to provide template or skip templated files

-t accepts file name under tune/templates with or without extension or path to file

```
-t work | -t work.toml | -t tune/templates/work.toml | -t /tmp/test.toml
```

Running commands after links
----------------------------
# this is TODO
In some cases, you need to run some commands after a link is created:
```
confine ln common .vimrc .vim

# init vundle repo
cd ~/.vim && git submodules update --init
```

This is getting old pretty quick.

The solution is postcreate:
```
cat tune/postcreate.toml
[common/.vim]
 - cd ~/.vim && git submodule update --init
 - echo 'Postupdate done'
```
