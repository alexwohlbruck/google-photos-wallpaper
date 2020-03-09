import pystray
from PIL import Image, ImageDraw

def open_options():
  print('Open options')

icon = pystray.Icon('test name')
icon.title = 'Google Photos Wallpaper'
icon.icon = Image.open('icon.png')
icon.menu = pystray.Menu(
  pystray.MenuItem(
    text = 'Options',
    action = open_options,
    default = True
  )
)

icon.run()