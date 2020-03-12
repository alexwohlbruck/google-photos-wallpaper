import webbrowser
import os
import wx
import random
import urllib.request

from src.google_api import GoogleApi

class OptionsFrame(wx.Frame):
	# Main options GUI

	def __init__(self, *args, **kw):
		super(OptionsFrame, self).__init__(*args, **kw)

		panel = wx.Panel(self)

		sign_in_button = wx.Button(panel, label='Sign in with Google')
		self.Bind(wx.EVT_BUTTON, self.sign_in, sign_in_button)

		set_wallpaper_button = wx.Button(panel, label='Set wallpaper', pos=(0, 30))
		self.Bind(wx.EVT_BUTTON, self.set_wallpaper, set_wallpaper_button)

		# self.makeMenuBar()

		self.favorites = None


	def sign_in(self, event):

		self.favorites = GoogleApi.get_favorites()


	def set_wallpaper(self, event):
		
		if (self.favorites):

			# Pick random photo from favorites
			image_url = random.choice(self.favorites['mediaItems'])['baseUrl']
			urllib.request.urlretrieve(image_url, "wall.jpg")
			path = os.path.abspath("wall.jpg")

			# Set wallpaper
			import ctypes
			SPI = 20
			SPIF = 2
			ctypes.windll.user32.SystemParametersInfoW(SPI, 0, path, SPIF)

	"""
	def makeMenuBar(self):
		fileMenu = wx.Menu()
		helloItem = fileMenu.Append(-1, "&Hello...\tCtrl-H")
		fileMenu.AppendSeparator()
		exitItem = fileMenu.Append(wx.ID_EXIT)

		helpMenu = wx.Menu()
		aboutItem = helpMenu.Append(wx.ID_ABOUT)

		menuBar = wx.MenuBar()
		menuBar.Append(fileMenu, "&File")
		menuBar.Append(helpMenu, "&Help")

		self.SetMenuBar(menuBar)

		self.Bind(wx.EVT_MENU, self.OnHello, helloItem)
		self.Bind(wx.EVT_MENU, self.OnExit,  exitItem)
		self.Bind(wx.EVT_MENU, self.OnAbout, aboutItem)
	"""

	def OnExit(self, event):
		self.Close(True)

	def OnHello(self, event):
		wx.MessageBox("Hello again from wxPython")

	def OnAbout(self, event):
		wx.MessageBox(
			"This is a wxPython Hello World sample",
			"About Hello World 2",
			wx.OK|wx.ICON_INFORMATION
		)
