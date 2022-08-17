from sys import platform
import ctypes

def set_wallpaper(path):
  if platform == 'darwin':
    # Mac
    os.system('osascript -e \'tell application "Finder" to set desktop picture to POSIX file "' + path + '"\'')
  elif platform == 'win32':
    # Windows
    ctypes.windll.user32.SystemParametersInfoW(20, 0, path, 0)
  elif platform == 'linux' or platform == 'linux2':
    # Linux
    os.system('gsettings set org.gnome.desktop.background picture-uri file://' + path)
    