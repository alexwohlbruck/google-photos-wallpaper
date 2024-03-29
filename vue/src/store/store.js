import Vue from 'vue'
import Vuex from 'vuex'

Vue.use(Vuex)

export const store = new Vuex.Store({
    state: {
        favorites: [],
        albums: [],
        options: {
            currentWallpaper: {},
            selectedAlbums: []
        }
    },

    mutations: {
        setCurrentWallpaper(state, { mediaItem }) {
            state.options.currentWallpaper = mediaItem;
        },
        setFavorites(state, { mediaItems }) {
            state.favorites = mediaItems;
        },
        setAlbums(state, { albums }) {
            state.albums = albums;
        },
        setAlbum(state, { albumId, mediaItems }) {
            state.albums = state.albums.map(album => {
                if (album.id == albumId) {
                    // album.mediaItems = mediaItems;
                    Vue.set(album, 'mediaItems', mediaItems);
                }
                return album;
            });
        },
        setUserOptions(state, { options }) {
            options.selectedAlbums = options.selectedAlbums.map(a => a.id);
            state.options = options;
        },
        setSystemInfo(state, { systemInfo }) {
            state.systemInfo = systemInfo;
        },
    },

    actions: {
        setWallpaper({ commit }, { mediaItem, source }) {
            mediaItem.source = {
                id: source.id,
                title: source.title
            };
            return new Promise((res) => {
                window.eel.set_wallpaper(mediaItem)(() => {
                    commit('setCurrentWallpaper', { mediaItem });
                    res();
                });
            });
        },
        setWallpaperByDirection({ commit }, { direction }) {
            return new Promise((res => {
                window.eel.set_wallpaper_by_direction(direction)(newWallpaper => {
                    commit('setCurrentWallpaper', { mediaItem: newWallpaper });
                    res();
                });
            }));
        },
        setWallpaperRandom({ commit }) {
            return new Promise((res => {
                window.eel.set_wallpaper_random()(newWallpaper => {
                    commit('setCurrentWallpaper', { mediaItem: newWallpaper });
                    res();
                })
            }));
        },
        getFavorites({ commit }) {
            window.eel.get_favorites()(({ mediaItems }) => {
                commit('setFavorites', { mediaItems });
            });
        },
        getAlbums({ commit }) {
            window.eel.get_albums()(({ albums }) => {
                commit('setAlbums', { albums });
            });
        },
        getAlbum({ commit }, { albumId }) {
            window.eel.get_album_media_items(albumId)(({ mediaItems }) => {
                commit('setAlbum', {
                    albumId,
                    mediaItems
                });
            });
        },
        getUserOptions({ commit }) {
            return new Promise((res) => {
                window.eel.get_user_options()(options => {

                    delete options.currentWallpaper.baseUrl; // Assume baseUrl is invalid

                    // Set already available data
                    commit('setUserOptions', { options });

                    // Save source data from original object
                    let source = options.currentWallpaper.source;

                    // Retrieve mediaItem again to get new baseUrl
                    window.eel.get_media_item(options.currentWallpaper.id)(mediaItem => {
                        mediaItem.source = source;
                        commit('setCurrentWallpaper', { mediaItem });
                        res();
                    });
                });
            });
        },
        setSchedule(context, { interval, unit }) {
            return new Promise((res) => {
                window.eel.set_schedule(interval, unit)(() => {
                    res();
                });
            });
        },
        setSelectedAlbums(context, { selectedAlbums }) {
            return new Promise((res) => {
                window.eel.set_selected_albums(selectedAlbums)(() => {
                    res();
                });
            });
        }
    }
});
