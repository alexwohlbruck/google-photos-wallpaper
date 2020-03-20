import Vue from 'vue'
import Vuex from 'vuex'

Vue.use(Vuex)

export const store = new Vuex.Store({
    state: {
        selectedAlbums: [],
		currentWallpaper: {},
		favorites: [],
		albums: []
    },

    getters: {

    },

    mutations: {
        setCurrentWallpaper (state, { mediaItem }) {
            state.currentWallpaper = mediaItem
        },
        setFavorites (state, { mediaItems }) {
            state.favorites = mediaItems;
        },
        setAlbums (state, { albums }) {
            state.albums = albums;
        },
        setAlbum(state, { albumId, mediaItems }) {
            state.albums = state.albums.map(album => {
                if (album.id == albumId) {
                    // album.mediaItems = mediaItems;
                    Vue.set(album, 'mediaItems', mediaItems)
                }
                return album;
            })
        }
    },

    actions: {
        getCurrentWallpaper ({ commit }) {
            return new Promise(( res ) => {
                window.eel.get_current_wallpaper()(mediaItem => {

                    // Save source data from original object
                    let source = mediaItem.source;
    
                    // Retrieve mediaItem again to get new baseUrl
                    window.eel.get_media_item(mediaItem.id)(mediaItem => {
                        mediaItem.source = source;
                        commit('setCurrentWallpaper', { mediaItem });
                        res();
                    });
                });
            })
        },
        setWallpaper ({ commit }, { mediaItem }) {
            return new Promise(( res ) => {
                window.eel.set_wallpaper(mediaItem)(() => {
                    commit('setCurrentWallpaper', { mediaItem });
                    res();
                });
            });
        },
        getFavorites ({ commit }) {
            window.eel.get_favorites()(({ mediaItems }) => {
                commit('setFavorites', { mediaItems });
            });
        },
        getAlbums ({ commit }) {
            window.eel.get_albums()(({ albums }) => {
                commit('setAlbums', { albums });
            });
        },
        getAlbum ({ commit }, { albumId }) {
            window.eel.get_album_media_items(albumId)(({ mediaItems }) => {
				commit('setAlbum', {
                    albumId,
                    mediaItems
                })
			})
        }
    }
})