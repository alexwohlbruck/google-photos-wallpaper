import eel
from src.google_api import GoogleApi

# TODO: Potentially redundant.
# TODO: Can just import eel in GoogleApi class and decorate that method
@eel.expose
def is_authenticated():
    return GoogleApi.is_authenicated()

@eel.expose
def sign_in():
    GoogleApi.ensure_valid_token()