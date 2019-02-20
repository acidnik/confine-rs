Confine - config manager
========================

OVERVIEW
--------
```
USAGE:
    confine [FLAGS] [OPTIONS] [SUBCOMMAND]

FLAGS:
    -n, --dry        dry run
    -h, --help       Prints help information
    -q               be quiet
    -V, --version    Prints version information

OPTIONS:
    -r <root>        config storage root [default: .]

SUBCOMMANDS:
    help      Prints this message or the help of the given subcommand(s)
    link      create symlink
    move      move file under config control
    undo      undo symlinking, restore original files
    delete    remove symlink and/or source file
```

USAGE
-----
To start, create a directory where your dotfiles will be stored
```
$ cd && mkdir confine && cd confine
```

Then, create directory for group. Each config file will belong to a certain group
```
$ mkdir common
```

Now you can move some existing files to that group
```
$ confine move common ~/.bashrc ~/.vimrc ~/.vim

$ ls -ld ~/.bashrc ~/.vim ~/.vimrc
lrwxrwxrwx  /home/user/.bashrc -> /home/user/confine/common/.bashrc
lrwxrwxrwx  /home/user/.vim -> /home/user/confine/common/.vim/
lrwxrwxrwx  /home/user/.vimrc -> /home/user/confine/common/.vimrc

```

VCS is not handled by confine, so it's up to you:
```
$ git init . && git add . && git commit -m 'initial'
# add remote and push
```

Next thing you probably want to do is restore you configuration on another machine:
```
# git clone ...; cd confine

# create links for all files in common
$ confine link common

# or only some files
$ confine link common .bashrc .tmux.conf

# the same but utilizing the power of tab-completion
$ confine link common/.bashrc common/.tmux.conf
```

If file in ~/ exists, it will be moved to backup/{hostname}/.bashrc before overwriting.

Next thing you probably don't want to do (but anyway there's an option to do so) is to undo link and replace it with solid file
```
confine undo common .bashrc
ls -l ~/.bashrc
-rw-r--r--  /home/user/.bashrc
```

The final stage of dotfile lifecycle is obsolesence. When you moved, say, from ack-grep to ag to ripgrep, you leave behind config files that you don't want anymore. This is the time to delete them for good
```
$ confine delete common .ackrc .agignore
# dont forget to git commit and git push
```

Or maybe you just want to remove the link, but want to keep the original file? This may be a good reason to move it to another group, but for now let's just delete the link
```
# rm is alias for delete
$ confine rm -l common/.zshrc
# or, you know, just rm ~/.zshrc ;)
```

TEMPLATES
---------
Some config files, such as .ripgreprc, don't allow using env variables or shell globbing, so there's no easy way
to set path to ignore file:
```
cat ~/.config/ripgrep/config 
--ignore-file=/Users/nikita/.config/ripgrep/ignore
```
Ugh, the path to home dir varies on different machines

Another example is .gitconfig. Some people want different settings for name and e-mail at home and work machines. Although there's a way to circumvent this problem with some git-config-foo, another way would be using a templates.

Templates are toml files that stored in directory `tune/templates`

```
$ cd confine
cat common/.config/ripgrep/config
--ignore-file={{HOME}}/.config/ripgrep/ignore

$ cat tune/templates/home.toml
["common/.gitconfig"]
GIT_USER="Nikita Bilous"
GIT_EMAIL="nikita@bilous.me"

["common/.config/ripgrep/config"]
```
Note that the section for ripgrep is empty. It's there to let confine know that the file should be processed. The only variable we are going to substitute is `{{HOME}}` which is defined in runtime by confine itself.

Now we can create links:
```
confine ln common/.config/ripgrep/config -t home
```

Templates are processed using [tera](https://crates.io/crates/tera) engine and stored in `tune/templates/processed`
```
$ ls -l ~/.config/ripgrep/
lrwxr-xr-x   config@ -> /Users/user/confne/tune/templates/processed/common/.config/ripgrep/config
```
