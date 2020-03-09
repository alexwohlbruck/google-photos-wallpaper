import wx
import pystray

from PIL import Image
from ui.options import OptionsFrame

app = wx.App()

def open_ui():
	frm = OptionsFrame(None, title='Hello world')
	frm.Show()
	app.MainLoop()

# Create system tray icon
icon = pystray.Icon('test name')
icon.title = 'Google Photos Wallpaper'
icon.icon = Image.open('icon.png')
icon.menu = pystray.Menu(
	pystray.MenuItem(
		text = 'Options',
		action = open_ui,
		default = True
	)
)

icon.run()