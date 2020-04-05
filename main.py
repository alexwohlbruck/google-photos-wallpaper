import eel
from PIL import Image
import pystray

import threading
import schedule
import time

from src.scheduler import Scheduler
from src.options import Options
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
		),
		pystray.MenuItem(
			text = 'Next photo',
			action = Options.set_wallpaper_next
		),
		pystray.MenuItem(
			text = 'Previous photo',
			action = Options.set_wallpaper_prev
		),
		pystray.MenuItem(
			text = 'Random photo',
			action = Options.set_wallpaper_random
		)
	)
		
	icon.run()

if __name__ == '__main__':
	# run_systray_icon()
	thread = threading.Thread(target = open_ui)
	thread.start()

	Scheduler.start()
	print(eel._js_functions)

	time.sleep(5)

	print(eel._js_functions)