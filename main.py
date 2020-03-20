import eel
from PIL import Image
import pystray

APP_TITLE = 'Google Photos Wallpaper'

import src.bridge
	
def open_ui():
	eel.init('web')
	eel.start('index.html', port=8686)
	
# Create system tray icon
icon = pystray.Icon('test name')
icon.title = APP_TITLE
icon.icon = Image.open('src/icon.png')
icon.menu = pystray.Menu(
	pystray.MenuItem(
		text = 'Options',
		action = open_ui,
		default = True
	)
)

# open_ui()
icon.run()
