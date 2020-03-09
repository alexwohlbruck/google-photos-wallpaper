import requests
import json
import webbrowser
import threading
import socket

from flask import Flask, redirect, request, url_for

import wx
import google.oauth2.credentials
from oauthlib.oauth2 import WebApplicationClient

import src.config as config


# Create a web server that opens the Google oAuth login screen
# and retrieves access and refresh tokens
		
app = Flask('Oauth')
oauthclient = WebApplicationClient(config.GOOGLE_CLIENT_ID)
google_provider_cfg = requests.get('https://accounts.google.com/.well-known/openid-configuration').json()

@app.route('/oauth')
def sign_in():
	request_uri = oauthclient.prepare_request_uri(
		google_provider_cfg['authorization_endpoint'],
		redirect_uri = request.base_url + '/callback',
		scope = ['https://www.googleapis.com/auth/photoslibrary.readonly']
	)

	return redirect(request_uri)

@app.route('/oauth/callback')
def callback():
	code = request.args.get('code')

	print(request.base_url)
	print(request.url)

	token_url, headers, body = oauthclient.prepare_token_request(
		google_provider_cfg['token_endpoint'],
		authorization_response = request.url.replace('http://', ''),
		redirect_url = request.base_url.replace('http://', ''),
		code = code
	)

	token_response = requests.post(
		token_url,
		headers = headers,
		data = body,
		auth = (config.GOOGLE_CLIENT_ID, config.GOOGLE_CLIENT_SECRET)
	)

	oauthclient.parse_request_body_response(json.dumps(token_response.json()))

	userinfo_endpoint = google_provider_cfg["userinfo_endpoint"]
	uri, headers, body = oauthclient.add_token(userinfo_endpoint)
	userinfo_response = requests.get(uri, headers=headers, data=body)

	print(userinfo_response)

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

		# Get unused TCL port
		s = socket.socket()
		s.bind(('', 0))
		port = s.getsockname()[1]

		threading.Thread(target=app.run, kwargs=dict(host='127.0.0.1', port=port, ssl_context='adhoc')).start()
		webbrowser.open('http://127.0.0.1:' + str(port) + '/oauth')

		""" 
		credentials = google.oauth2.credentials.Credentials(
			'access_token'
			refresh_token='refresh_token',
			token_uri='token_uri'
			client_id
		)
		"""

		""" client_id = config.GOOGLE_CLIENT_ID
		client_secret = config.GOOGLE_CLIENT_SECRET
		scopes = ['https://www.googleapis.com/auth/photoslibrary.readonly']

		credentials = google_auth_oauthlib.get_user_credentials(
			scopes, client_id, client_secret
		)

		print(credentials) """

		# 1. Open the link.
		# 2. Authorize the application to have access to your account.
		# 3. Copy and paste the authorization code to the prompt.

		# Use the credentials to construct a client for Google APIs.
		""" from google.cloud import bigquery

		bigquery_client = bigquery.Client(
			credentials=credentials, project="your-project-id"
		)
		print(list(bigquery_client.query("SELECT 1").result())) """

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
