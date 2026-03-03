# Filesearch

This is a command-line file search application originally written in C, now written with Rust. The application searches for a specified filename or subdirectory in a directory and all its subdirectories and provides a list of matching files or subdirectories. It supports threading, logfiles, wildcards, case-sensitivity, and more.


## Credits: 
-  GitHub: @nreef12 for building the ARM64 version for macOS
-  Friend: @caden_tm for opening the doors to Intel macOS

## Compatibility
![ScreenshotsStacked](https://github.com/user-attachments/assets/bc279ff5-a4dd-4819-9fb1-cc58186152e0)
<div align=center>

|OS  | 64-bit | 32-bit | ARM64 |
|-|-|-|-|
|Windows|Yes|Yes|Yes
|macOS|Yes (Intel)|Not Planned|Yes (Silicon)|
|Linux|Yes |Yes|Yes

</div>

Windows, macOS and Linux each have their very own version of Filesearch - not to mention it's also built for different CPU architectures, too! So whether you're using Windows 7 on a 32-bit CPU, macOS 26 on a M5 Mac Mini, or Damn Small Linux on a MacBook Pro from 2009, it'll run.

The minimum version of Windows needed to run either the 32-bit or 64-bit is Windows 7 SP1. I am actively working on fixing that and will update the existing files silently. You'll know because this message will be gone - you aren't seeing things... or are you? :) I am—


## Features
- Threading
- Logging
- Wildcards
- File searching
- Subdirectory searching
- Switch between two search patterns (BFS / DFS)
- Case-sensitivity

## Usage
To demonstrate how simple it is to use this, I came up with 12 examples in four groups of complexity:

1. Rush — on-the-go
- filesearch /fm x.py
- filesearch -f favicon.png ./www
- filesearch -d *.app /Applications
2. Simple — productivity
- filesearch /FM *.pptx C:\\Users\\Randell\\OneDrive\\Documents
- filesearch --files income-2023-*.ods
- filesearch --folders wiki /home/server_user/git/www/ --log ~/current-wikis.log
3. Specific — you know what you want
- filesearch /FM recording-??-??-2004.avi /mnt/nas-backups/pre2005/camcorder/Vacations/Germany
- filesearch -f rufus-?-??p.exe ..\Downloads
- filesearch  -f ??-??-1996_stevie+chris.mp? B:\\pre2005\\camcorder\\mixed-graduations
4. Forgetful — you forget 80% of what you want
- filesearch -d * C:\\Users\\
- filesearch /FM *-stable.tar.gz .
- filesearch /FM dirent.h /.


## Installation
Installation is simple. You copy the binary you download to somewhere in your PATH. This could be a custom folder you _add_ to PATH yourself, or a system folder for executables. I prefer the latter myself. Heres how to do it:
### Linux
On Linux it's easy. You go to the path you downloaded the zip, extract it and copy `filesearch` to `/usr/bin` by running:
`sudo cp ./filesearch /usr/bin`
or for limited users, you can install it for **your user only**:
```
mkdir ~/.local ~/.local/bin
cp ./filesearch ~/.local/bin
echo 'export PATH="$PATH:~/.local/bin"' >> ~/.bashrc
source ~/.bashrc
```

### macOS
Installation is... **_less easy_** but still possible. You can not directly write to `/bin` or `/usr/bin` even _with_ sudo! So, to install **system-wide** we need to do some more trickery. First extract the binary to your Downloads folder, then run these commands:
```
sudo mkdir -p /usr/local/filesearchbin
sudo chmod 755 /usr/local/filesearchbin
echo "/usr/local/filesearchbin" | sudo tee /etc/paths.d/filesearchbin
sudo cp ./filesearch /usr/local/filesearchbin
```
That should do it. Probably. If not open an issue if I didn't already catch it. Same for any of these installation guides... Aannyway to install it for _just_ your user, no one else, incase you can't use sudo, run:
```
mkdir ~/filesearchbin
echo 'export PATH="$PATH:~/filesearchbin"' >> ~/.zshrc
source ~/.zshrc
```

### Windows
Ah... Windows... You either love it or hate it, but for me it's both. Fight me.
To install Filesearch system-wide for Windows, first make sure you extract the binary to your downloads folder. After that, just:

1. Click on the Start Button (<img width="16" height="16" alt="" src="https://github.com/user-attachments/assets/322977d9-15cc-4f49-a608-c6b1de629689" />)
2. Search "cmd" and click 'Run as administrator'
<img width="650" height="550" alt="image" src="https://github.com/user-attachments/assets/f55d5452-9208-49e1-95dd-b3d1b9f16ca3" />

If prompted, press 'Yes' or enter your password.
3. Navigate to your Downloads folder by using `cd` followed by your user folder's path. for example, my folder would be `C:\\Users\\Emmet\\`. Then you just go to the downloads folder, or wherever you extracted the binary to.
4. Run the following commands:
```batch
cp .\\filesearch.exe C:\\Windows\\System32\\
filesearch
```
This will install the binary and verify it runs.

If you instead want it to be just for your user, instead run:
```batch
mkdir "%LOCALAPPDATA%\\Programs\\FilesearchBin"
move ".\\filesearch.exe" "%LOCALAPPDATA%\\Programs\\FilesearchBin"
setx PATH "%PATH%;%LOCALAPPDATA%\\Programs\\FilesearchBin"
```

> Note: If the method you use includes adding the binary path to your system **or** user PATH variable, the shell MUST be restarted for it to take effect. This does NOT apply if you copied the binary to System32 on Windows (because it's already in PATH). You can also just run your shell again. For example, if you use `fish` as your shell and just added filesearch to your path, just run `fish` to open a new instance and it will use the new path. This isn't recommended, but it is an alternative for those like me who are lazy sometimes. 

## Attribution

Contributors who help test and build versions for macOS and Linux will be credited for their contributions in this README and the project's GitHub repository.

## License

This project is licensed under the MIT License - see the [LICENSE](LICENSE) file for details.

## Contact

If you have any questions or need further assistance, feel free to open an issue in the repository.
