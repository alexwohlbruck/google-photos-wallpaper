import webbrowser
import os

import wx
# import google.oauth2.credentials
# from oauthlib.oauth2 import WebApplicationClient
from google_auth_oauthlib.flow import Flow

import src.config as config

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

	def sign_in(self, event):

		flow = Flow.from_client_secrets_file(
			os.path.abspath('src/client_secrets.json'),
			scopes=[
				'openid',
				'https://www.googleapis.com/auth/userinfo.email',
				'https://www.googleapis.com/auth/photoslibrary.readonly'
			],
			redirect_uri='urn:ietf:wg:oauth:2.0:oob'
		)

		auth_url, _ = flow.authorization_url(prompt='consent')
		webbrowser.open(auth_url)

		codeInputDialog = wx.TextEntryDialog(None, 'Enter your authorization code', 'Google sign in')
		
		if codeInputDialog.ShowModal() == wx.ID_OK:

			
			# The user will get an authorization code. This code is used to get the
			# access token.
			code = codeInputDialog.GetValue()
			flow.fetch_token(code=code)

			# You can use flow.credentials, or you can just get a requests session
			# using flow.authorized_session.
			session = flow.authorized_session()


			favorites = session.post(
				'https://photoslibrary.googleapis.com/v1/mediaItems:search',
				json = {
					'filters': {
						'featureFilter': {
							'includedFeatures': ['FAVORITES']
						}
					}
				}
			).json()

			print(favorites)
		

	def set_wallpaper(self, event):

		# Set wallpaper
		import ctypes
		SPI = 20
		SPIF = 2
		ctypes.windll.user32.SystemParametersInfoW(SPI, 0, 'C:/Users/alexw/Downloads/test.jpg', SPIF)

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
