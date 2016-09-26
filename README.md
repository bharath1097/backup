# Backup

`backup` is a Linux backup system, designed to use in local backup or to sync with Google Drive.

## Build the source

- Install dependecies: `git`, `rust`, `cargo`, `make`.
- Clone the repository:
```
git clone https://github.com/juchiast/backup.git
```
- Configure the system:
```
./configure
```
- Compile:
```
make
```

## Using the software

- Edit data file of `backup`, the default data file located at `data/file`, view `file.example` for an example data file.
- Run: `./bk`
- The program will output symbolic links to original files in 2 folders: `output` and `inc`. `output` contains all the files you want to backup, while `inc` contains files that was changed since the last snapshot. You can save the current snapshot by `chdir` into `./db` commit the change:
```
git add file
git commit -m "some commit messages"
```
- Copy files to your backup device:
```
cp -RL output/file <your backup device>
```
Or copy only changed files:
```
cp -RL inc/file <your backup device>
```

## Multiple data files

The program supports multiple data files in `./data` folder. To generate output for a specific data file, run:
```
./bk <your data file's name>
```
Note: Do not include `./data/` in your data file's name. The outputs will be placed in `output/<your data file's name>` and `inc/<your data file's name>`.

## License

The sotfware is licensed under the MIT License. See `LICENSE` for detail.

## Contributing

All contributions are welcomed. You can freely report bugs, request new features, change the code, push pull requests and more.
