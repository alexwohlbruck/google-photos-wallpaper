import json
from src.google_api import GoogleApi

def find_in_list_by_val(list_, key, val):
    return next((d for d in list_ if d[key] == val), None)
    
def find_index_in_list_by_val(list_, key, val):
    return next((i for i, d in enumerate(list_) if d[key] == val), None)

# Read and update user-set options for the app
class Options():

    OPTIONS_PATH = 'storage/options.json'

    @classmethod
    def get_current_wallpaper(cls):
        # Get the set wallpaper data
        
        # NOTE: Access to `baseUrl` will expire after session ends,
        # Use GoogleApi.get_media_item to refresh it

        with open(cls.OPTIONS_PATH, 'r') as f:
            options = json.load(f)
            f.close()

        return options.get('currentWallpaper')

    @classmethod
    def set_current_wallpaper(cls, media_item):
        # Update the current wallpaper data
        with open(cls.OPTIONS_PATH, 'r') as f:
            options = json.load(f)
            f.close()

        options['currentWallpaper'] = media_item

        with open(cls.OPTIONS_PATH, 'w') as f:
            json.dump(options, f)
            f.close()

    @classmethod
    def set_wallpaper_next(cls):
        with open(cls.OPTIONS_PATH, 'r') as f:
            options = json.load(f)
            f.close()

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

        index = current_wall_index + 1

        if index == len(current_album_items):
            # Out of bounds
            # TODO: Change to next album when going out of bounds
            return
        else:
            new_wall = current_album_items[index]
            new_wall['source'] = current_album
            cls.set_current_wallpaper(new_wall)
            print(new_wall)
            return new_wall
    
    @classmethod
    def set_wallpaper_random(cls):
        return
    
    @classmethod
    def set_wallpaper_random(cls):
        return
    
    @classmethod
    def get_user_options(cls):
        # Return the entire options config
        
        with open(cls.OPTIONS_PATH, 'r') as f:
            options = json.load(f)
            f.close()
        return options
    
    @classmethod
    def set_selected_albums(cls, selected_items):
        with open(cls.OPTIONS_PATH, 'r') as f:
            options = json.load(f)
            f.close()

        original_selection = options.get('selectedAlbums', [])
        new_selection = []

        # Iterate through new selection
        for album_id in selected_items:
            # Search in original selection for item containing album_id
            search = find_in_list_by_val(original_selection, 'id', album_id)

            if search == None:
                # User has selected a new album
                if album_id == 'FAVORITES':
                    media_items = GoogleApi.get_all_favorites()
                else:
                    media_items = GoogleApi.get_all_album_media_items(album_id)
                
                # Remove these baseUrl and productUrl - not needed for storage
                for item in media_items:
                    item.pop('baseUrl')
                    item.pop('productUrl')
                
                new_selection.append({
                    'id': album_id,
                    'mediaItems': media_items
                })
            else:
                # Album already selected, use old data
                new_selection.append(search)
        
        options['selectedAlbums'] = new_selection

        with open(cls.OPTIONS_PATH, 'w') as f:
            json.dump(options, f)
            f.close()
