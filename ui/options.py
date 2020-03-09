import wx

class OptionsFrame(wx.Frame):
	# Main options GUI

	def __init__(self, *args, **kw):
		super(OptionsFrame, self).__init__(*args, **kw)

		panel = wx.Panel(self)

		sign_in_button = wx.Button(panel, label='Sign in with Google')
		
		

		# self.makeMenuBar()

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
