import pickle
import os
import eel

from googleapiclient.discovery import build
from googleapiclient.errors import HttpError
from google_auth_oauthlib.flow import Flow
from google_auth_oauthlib.flow import InstalledAppFlow
from google.auth.transport.requests import Request

PICKLE_PATH = 'storage/token.pickle'

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
                    os.path.abspath('src/client_secrets.json'),
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
        cls.photos = build('photoslibrary', 'v1', credentials = creds)

    @classmethod
    def get_favorites(cls):
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
            'pageSize': 100
        }
        return cls.photos.mediaItems().search(body = body).execute()

    @classmethod
    def get_albums(cls):
        cls.ensure_valid_token()
        body = {
            'pageSize': 100
        }
        return cls.photos.albums().list(pageSize = 50).execute()

    @classmethod
    def get_album_media_items(cls, album_id):
        cls.ensure_valid_token()
        body = {
            'albumId': album_id,
            'pageSize': 100
        }
        return cls.photos.mediaItems().search(body = body).execute()