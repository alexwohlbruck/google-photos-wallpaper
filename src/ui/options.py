import wx
import google.oauth2.credentials
from oauthlib.oauth2 import WebApplicationClient

import requests
import webbrowser
import threading
import socket

from flask import Flask, redirect, request, url_for

import src.config as config




# Create a web server that opens the Google oAuth login screen
# and retrieves access and refresh tokens
		
app = Flask('Oauth')
oauthclient = WebApplicationClient(config.google_client_id)

@app.route('/')
def sign_in():
	request_uri = oauthclient.prepare_request_uri(
		'https://accounts.google.com/o/oauth2/v2/auth',
		redirect_uri = request.base_url + '/callback',
		scope = ['https://www.googleapis.com/auth/photoslibrary.readonly']
	)

	return redirect(request_uri)


class OptionsFrame(wx.Frame):
	# Main options GUI

	def __init__(self, *args, **kw):
		super(OptionsFrame, self).__init__(*args, **kw)

		panel = wx.Panel(self)

		sign_in_button = wx.Button(panel, label='Sign in with Google')

		self.Bind(wx.EVT_BUTTON, self.sign_in, sign_in_button)

		# self.makeMenuBar()

	def sign_in(self, event):

		# Get unused TCL port
		s = socket.socket()
		s.bind(('', 0))
		port = s.getsockname()[1]

		threading.Thread(target=app.run, kwargs=dict(host='127.0.0.1', port=port)).start()
		webbrowser.open('http://127.0.0.1:' + str(port))

		""" 
		credentials = google.oauth2.credentials.Credentials(
			'access_token'
			refresh_token='refresh_token',
			token_uri='token_uri'
			client_id
		)
		"""

		""" client_id = config.google_client_id
		client_secret = config.google_client_secret
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
