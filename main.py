import eel
from PIL import Image
import pystray

import threading
import schedule
import time

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

def job():
	print('test')

from src.options import Options
def run_wallpaper_scheduler(name):
	schedule.every(10).seconds.do(Options.set_wallpaper_random)
	while True:
		schedule.run_pending()
		time.sleep(1)

if __name__ == '__main__':
	scheduler = threading.Thread(target = run_wallpaper_scheduler, args = (1,))
	scheduler.start()
	# icon = threading.Thread(target = run_systray_icon, args = (1,))
	run_systray_icon()