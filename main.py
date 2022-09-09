import time
import bottle_websocket
import eel
from PIL import Image
import pystray

import threading
import schedule
import time

from src.scheduler import Scheduler
from src.options import Options
from src.wallpaper import Wallpaper
import src.bridge
from src.helpers import resource_path

from pyupdater.client import Client
from src.lib.client_config import ClientConfig

APP_NAME = 'Google Photos Wallpaper'
APP_VERSION = '0.0.1'

# Pyupdater download status callback
def print_status_info(info):
	total = info.get(u'total')
	downloaded = info.get(u'downloaded')
	status = info.get(u'status')
	print(downloaded, total, status)

# print('Initializing client')
# client = Client(
# 	ClientConfig(), refresh=True, progress_hooks=[print_status_info]
# )
# print('Client initialized')

# print('Checking for updates')
# app_update = client.update_check(APP_NAME, APP_VERSION)

# if app_update is not None:
# 	print('Update available, downloading')
# 	app_update.download(background=True)

# 	if app_update.is_downloaded():
# 		print('Downloaded update, restarting')
# 		app_update.extract_restart()
# else:
# 	print("No updates")

def open_ui():
	eel.init('web')
	eel.start('index.html', port=8686)

def run_systray_icon():
	icon = pystray.Icon('test name')
	icon.title = APP_NAME
	icon.icon = Image.open(resource_path('src/icon.png'))
	icon.menu = pystray.Menu(
		pystray.MenuItem(
			text = 'Options',
			action = open_ui,
			default = True
		),
		pystray.MenuItem(
			text = 'Next photo',
			action = Wallpaper.set_wallpaper_next
		),
		pystray.MenuItem(
			text = 'Previous photo',
			action = Wallpaper.set_wallpaper_prev
		),
		pystray.MenuItem(
			text = 'Random photo',
			action = Wallpaper.set_wallpaper_random
		)
	)
		
	icon.run()

if __name__ == '__main__':
	print('running')
	time.sleep(1)

	Scheduler.start()
# 	# ui = threading.Thread(target = open_ui)
# 	# ui.start()
	run_systray_icon()
