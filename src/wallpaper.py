from sys import platform
import os
import ctypes
import urllib
import random
import eel

from src.options import Options
from src.google_api import GoogleApi
from src.helpers import resource_path, find_in_list_by_val, find_index_in_list_by_val

def get_large_url(media_item):
    # Helper function to get the max size image url
    try:
        width = media_item['mediaMetadata']['width']
        height = media_item['mediaMetadata']['height']
    except (AttributeError, KeyError):
        width = height = '16383'

    return media_item['baseUrl'] + '=w' + width + '-h' + height

class Wallpaper():
	
	@classmethod
	def set_wallpaper(this, path):
		if platform == 'darwin':
			# Mac
			os.system('osascript -e \'tell application "Finder" to set desktop picture to POSIX file "' + path + '"\'')
		elif platform == 'win32':
			# Windows
			ctypes.windll.user32.SystemParametersInfoW(20, 0, path, 0)
		elif platform == 'linux' or platform == 'linux2':
			# Linux
			os.system('gsettings set org.gnome.desktop.background picture-uri file://' + path)

	@classmethod
	def get_current_wallpaper(this):
		options = this.get_user_options()

		selected_albums = options.get('selectedAlbums', None)
		new_album = random.choice(selected_albums)

		new_album_id = new_album.get('id')
		album_media_items = new_album.get('mediaItems', None)

		# TODO: Store album title in options.json instead of retrieving it each time
		if (new_album_id == FAVORITES):
			new_album['title'] = 'Favorites'
		else:
			new_album = GoogleApi.get_album(new_album_id)

		new_wall = random.choice(album_media_items)

		# Refresh base url
		new_wall = GoogleApi.get_media_item(new_wall.get('id'))
		new_album.pop('mediaItems', None)
		new_wall['source'] = new_album

		this.set_current_wallpaper(new_wall)
		return new_wall

	@classmethod
	def set_current_wallpaper(this, media_item):
		
		file_prefix = 'wall_'
		filename = file_prefix + media_item.get('filename')
		
		print('Setting wallpaper: ' + filename);
		
		# Delete files in /storage that are named wall_*
		for file in os.listdir(resource_path('storage')):
			if file.startswith(file_prefix):
				os.remove(resource_path('storage/' + file))

		# Update the current wallpaper data
		options = Options.get_user_options()

		# Download full res image and change wallpaper
		image_url = get_large_url(media_item)
		path = resource_path('storage/' + filename + '.jpg')
		urllib.request.urlretrieve(image_url, path)
		
		this.set_wallpaper(path)

		if (hasattr(eel, 'wallpaper_changed')):
			eel.wallpaper_changed(media_item)
			
		options['currentWallpaper'] = media_item

		Options.save_options(options)

	@classmethod
	def set_wallpaper_next(this):
		return this.set_wallpaper_by_direction(direction = 'next')
	
	@classmethod
	def set_wallpaper_prev(this):
		return this.set_wallpaper_by_direction(direction = 'prev')

	@classmethod
	def set_wallpaper_by_direction(this, direction = 'next'):

		options = Options.get_user_options()
		current_wall = options.get('currentWallpaper', None)

		if (current_wall == None):
			# No current wallpaper set
			return None

		current_album = current_wall.get('source')
		current_album_id = current_album.get('id')
		current_wall_id = current_wall.get('id')

		# Get the album that the current wallpaper is in
		current_album_items = find_in_list_by_val(
			options.get('selectedAlbums'),
			'id',
			current_album_id
		).get('mediaItems')

		# Get the index of the current wallpaper
		current_wall_index = find_index_in_list_by_val(
			current_album_items,
			'id',
			current_wall_id
		)

		if (direction == 'next'):
			index = current_wall_index + 1
		elif (direction == 'prev'):
			index = current_wall_index - 1

		if (index == len(current_album_items) and direction == 'next') or (index < 0 and direction == 'prev'):
			# Out of bounds
			# TODO: Change to next album when going out of bounds
			return
		else:
			new_wall_id = current_album_items[index].get('id')
			new_wall = GoogleApi.get_media_item(new_wall_id)
			new_wall['source'] = current_album

			this.set_current_wallpaper(new_wall)
			return new_wall
	
	@classmethod
	def set_wallpaper_random(this):

		options = Options.get_user_options()

		selected_albums = options.get('selectedAlbums', None)
		new_album = random.choice(selected_albums)

		new_album_id = new_album.get('id')
		album_media_items = new_album.get('mediaItems', None)

		# TODO: Store album title in options.json instead of retrieving it each time
		if (new_album_id == 'FAVORITES'):
			new_album['title'] = 'Favorites'
		else:
			new_album = GoogleApi.get_album(new_album_id)

		new_wall = random.choice(album_media_items)

		# Refresh base url
		new_wall = GoogleApi.get_media_item(new_wall.get('id'))
		new_album.pop('mediaItems', None)
		new_wall['source'] = new_album

		this.set_current_wallpaper(new_wall)
		return new_wall
	