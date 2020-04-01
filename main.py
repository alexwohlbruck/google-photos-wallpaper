import eel
from PIL import Image
import pystray

import threading
import schedule
import time

from src.scheduler import Scheduler
import src.bridge

APP_TITLE = 'Google Photos Wallpaper'

def open_ui():
	eel.init('web')
	eel.start('index.html', port=8686)

def run_systray_icon():
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
		
	icon.run()

if __name__ == '__main__':
	Scheduler.start()
	run_systray_icon()