import pickle
import os
import eel

from googleapiclient.discovery import build
from googleapiclient.errors import HttpError
from google_auth_oauthlib.flow import Flow
from google_auth_oauthlib.flow import InstalledAppFlow
from google.auth.transport.requests import Request

from src.helpers import resource_path

PICKLE_PATH = resource_path('storage/token.pickle')

def deco(cls):
    """
    Initialize class with credentials if available, then build API services
    This method runs when the GoogleApi class is finished building
    """

    # Open Oauth credentials from stored file
    creds = None
    if os.path.exists(PICKLE_PATH):
        with open(PICKLE_PATH, 'rb') as token:
            cls.creds = pickle.load(token)
            cls.build_services(cls.creds)

    return cls

@deco
class GoogleApi():

    """
    Class used to get data from the Google API

    Attributes:
        creds (google.oauth2.credentials.Credentials): The OAuth 2.0 credentials for the user
        photos (Resource): Google photos API service - only to use after signing in
    """

    creds = None
    photos = None

    @classmethod
    def is_authenicated(cls):
        return cls.creds and cls.creds.valid

    @classmethod
    def ensure_valid_token(cls):
        
        """
        Make sure the current credentials have not expired
        If so, new ones will be generated either by user consent or with a refresh token
        """

        # If credentials aren't stored or have expired
        if not cls.is_authenicated():

            # Expired, get new access token
            if cls.creds and cls.creds.expired and cls.creds.refresh_token:
                cls.creds.refresh(Request())

            # Retrieve refresh token and other creds from Oauth flow
            else:
                flow = InstalledAppFlow.from_client_secrets_file(
                    resource_path('src/client_secrets.json'),# os.path.abspath(),
                    scopes = [
                        'openid',
                        'https://www.googleapis.com/auth/userinfo.email',
                        'https://www.googleapis.com/auth/photoslibrary.readonly'
                    ],
                    redirect_uri = 'urn:ietf:wg:oauth:2.0:oob:auto'
                )
                cls.creds = flow.run_local_server(
                    success_message = 'Signed in sucsessfully, you may close this window.',
                )

            # Save new credentials
            with open(PICKLE_PATH, 'wb') as token:
                pickle.dump(cls.creds, token)

            cls.build_services(cls.creds)
    
    @classmethod
    def build_services(cls, creds):
        """
        Rebuild API services for a new set of credentials
        """
        cls.photos = build('photoslibrary', 'v1', credentials=creds, static_discovery=False)

    @classmethod
    def get_favorites(cls, page_size=None, page_token=None):
        cls.ensure_valid_token()
        body = {
            'filters': {
                'featureFilter': {
                    'includedFeatures': ['FAVORITES']
                },
                'mediaTypeFilter': {
                    'mediaTypes': ['PHOTO']
                }
            },
            'pageSize': 100,
            'pageToken': page_token
        }
        try:
            return cls.photos.mediaItems().search(body = body).execute()
        except:
            return cls.get_favorites(page_size, page_token)

    @classmethod
    def get_all_favorites(cls, page_token = None):
        
        # TODO: This is practically the same code as get_all_album_media_items
        # TODO: Consider merging the two methods, as well as get_favorites and get_album_media_items
        # TODO: They are operating on the same API endpoint, it makes sense to merge them

        print('Get favorites: page token: ' + str(page_token))

        page = cls.get_favorites(page_size = 100, page_token = page_token)
        media_items = page.get('mediaItems')
        next_page_token = page.get('nextPageToken', None)

        if next_page_token == None:
            return media_items
        else:
            next_page = cls.get_all_favorites(page_token = next_page_token)
            return media_items + next_page

    @classmethod
    def get_album(cls, album_id):
        cls.ensure_valid_token()
        try:
            return cls.photos.albums().get(albumId = album_id).execute()
        except:
            # Sometimes returns SSLError. This is a one-off error, try again
            return cls.get_album(album_id)

    @classmethod
    def get_albums(cls):
        cls.ensure_valid_token()
        body = {
            'pageSize': 100
        }
        try:
            return cls.photos.albums().list(pageSize = 50).execute()
        except:
            return cls.get_albums()

    @classmethod
    def get_album_media_items(cls, album_id, page_size = None, page_token = None):

        print('Getting media items: album: ' + album_id + ' page: ' + str(page_token))

        cls.ensure_valid_token()
        body = {
            'albumId': album_id,
            'pageSize': 100,
            'pageToken': page_token
        }
        try:
            return cls.photos.mediaItems().search(body = body).execute()
        except:
            return cls.get_album_media_items(album_id, page_size, page_token)

    @classmethod
    def get_all_album_media_items(cls, album_id, page_token = None):
        # Retreive entire album content recursively
        cls.ensure_valid_token()

        page = cls.get_album_media_items(album_id, page_size = 100, page_token = page_token)
        media_items = page.get('mediaItems')
        next_page_token = page.get('nextPageToken', None)

        if next_page_token == None:
            return media_items
        else:
            next_page = cls.get_all_album_media_items(album_id, page_token = next_page_token)
            return media_items + next_page

    @classmethod
    def get_media_item(cls, media_item_id):
        cls.ensure_valid_token()
        try:
            return cls.photos.mediaItems().get(mediaItemId = media_item_id).execute()
        except:
            return cls.get_media_item(media_item_id)