import os

def set_wallpaper(path):
  if os.name == 'posix':
    # Mac
    os.system('osascript -e \'tell application "Finder" to set desktop picture to POSIX file "' + path + '"\'')
  elif os.name == 'nt':
    # Windows
    os.system('wallpaper.exe ' + path)
  elif os.name == 'linux':
    # Linux
    os.system('gsettings set org.gnome.desktop.background picture-uri file://' + path)
    