import webbrowser
import os
import random
import time
import urllib.request

import wx
import wx.lib.agw.thumbnailctrl as TC

from src.google_api import GoogleApi

class OptionsFrame(wx.Frame):

	def __init__(self):
		wx.Frame.__init__(
			self, parent=None,
			title='Wallpaper options',
			size = wx.Size(550, 700),
			pos = wx.Point(900,300)
		)
		# self.fSizer = wx.BoxSizer(wx.VERTICAL)
		panel = OptionsPanel(self)
		# self.fSizer.Add(panel, 1)
		# self.SetSizer(self.fSizer)
		# self.Fit()
		self.Show()

class OptionsPanel(wx.Panel):
	# Main options GUI

	def __init__(self, parent):
		
		# Build layout
		wx.Panel.__init__(self, parent)
		self.mainSizer = wx.BoxSizer(wx.VERTICAL)
		self.frame = parent

		# Define layout widgets
		sign_in_button = wx.Button(self, label='Sign in with Google')
		self.Bind(wx.EVT_BUTTON, self.sign_in, sign_in_button)

		set_wallpaper_button = wx.Button(self, label='Set wallpaper')
		self.Bind(wx.EVT_BUTTON, self.set_wallpaper, set_wallpaper_button)

		thumb = TC.ThumbnailCtrl(self, imagehandler = TC.PILImageHandler)
		thumb.ShowDir(os.path.join(os.getcwd(), 'storage/thumbs'))

		# Add widgets to sizer
		self.mainSizer.Add(sign_in_button, 0)
		self.mainSizer.Add(set_wallpaper_button, 0)
		# self.layout.Add(thumb, 500, wx.EXPAND)

		self.SetSizer(self.mainSizer)


		# Rest of constructor
		# self.makeMenuBar()
		self.favorites = None


	def sign_in(self, event):

		self.favorites = GoogleApi.get_favorites()

	
	def show_thumbs(self):
		print('show thumbs')
		thumb = TC.ThumbnailCtrl(self, imagehandler = TC.PILImageHandler)
		thumb.ShowDir(os.path.join(os.getcwd(), 'storage/thumbs'))
		
		self.mainSizer.Add(thumb, 0, wx.EXPAND, wx.EXPAND)
		self.mainSizer.Update()

	def set_wallpaper(self, event):

		if (self.favorites):
			MAX_SIZE = '16383'
			THUMB_SIZE = '50'

			# Pick random photo from favorites
			image_url = random.choice(self.favorites['mediaItems'])['baseUrl']
			image_url += '=w' + MAX_SIZE + '-h' + MAX_SIZE

			# Download full res image
			# urllib.request.urlretrieve(image_url, 'storage/wall.jpg')
			# path = os.path.abspath('storage/wall.jpg')

			# for i, thumb in enumerate(self.favorites['mediaItems']):
			# 	image_url = thumb['baseUrl']
			# 	image_url += '=w' + THUMB_SIZE + '-h' + THUMB_SIZE

			# 	urllib.request.urlretrieve(image_url,
			# 		'storage/thumbs/' + thumb.get('filename', 'thumb' + str(i) + '.jpg')
			# 	)
			
			self.show_thumbs()

			# # Set wallpaper
			# import ctypes
			# SPI = 20
			# SPIF = 2
			# ctypes.windll.user32.SystemParametersInfoW(SPI, 0, path, SPIF)

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
