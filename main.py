import wx
import pystray

from PIL import Image
from src.ui.options import OptionsFrame

APP_TITLE = 'Google Photos Wallpaper'

app = wx.App()

def open_ui():
	frm = OptionsFrame(None, title = APP_TITLE, size = wx.Size(550, 700), pos = wx.Point(900,300))
	frm.Show()
	app.MainLoop()

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

# Open automatically for testing
# open_ui()
icon.run()
