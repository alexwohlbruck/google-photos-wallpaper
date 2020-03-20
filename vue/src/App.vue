<template lang="pug">
  v-app
    v-app-bar(
      app
      max-height='1000'
      flat
    )
      v-app-bar-nav-icon
      
    v-content
      v-container
        //- Current wallpaper preview
        v-row.mb-12.align-center
          v-col(cols='5')
            v-card.current-wallpaper(elevation='22')
              v-skeleton-loader(type='image' max-height='350')
              //- v-img(:src='' max-height='350')

          //- Current item details
          v-col.pb-10(cols='7')
            .pa-5
              h3.headline.font-weight-bold Current wallpaper
              v-breadcrumbs.px-1.pt-1.pb-2(:items='breadcrumbs')
                template(v-slot:divider)
                  v-icon mdi-chevron-right
              v-btn.ma-1(outlined) Previous
              v-btn.ma-1(outlined) Next
              v-btn.ma-1(outlined) Random

        //- Album list
        v-skeleton-loader(
          type='list-item@10'
          :loading='!(favorites.length && albums.length)'
        )
          h3.subtitle-1.font-weight-bold.ma-3 Selected albums

          v-expansion-panels.albums(accordion multiple)
            //- Favorites panel
            v-expansion-panel
              v-expansion-panel-header.py-0(v-slot='open')
                v-layout
                  v-checkbox(
                    label='Favorites'
                    :value='favorites'
                    @click.native='preventExpansion'
                  )
              v-expansion-panel-content
                v-row
                  v-col.pa-1(
                    cols='3' sm='2' md='1'
                    v-for='mediaItem in favorites'
                    :key='mediaItem.id'
                  )
                    v-card(@click='setWallpaper(mediaItem);')
                      v-img(
                        :src='mediaItem | medUrl'
                        :lazy-src='mediaItem | smallUrl'
                        aspect-ratio='1'
                      )
                        //- Loading overlay
                        v-overlay(absolute :value='mediaItem.loading == true')
                          v-progress-circular(indeterminate)
                        //- Current set wallpaper overlay
                        v-overlay(absolute color='blue' :value='mediaItem.id == currentWallpaper.id')
                          v-icon mdi-check

            //- Album panels list           
            v-expansion-panel(
              v-for='album in albums'
              :key='album.id'
              @change='loadAlbum(album.id)'
            )
              v-expansion-panel-header.py-0(v-slot='open')
                v-layout
                  v-checkbox(
                    v-model='selectedAlbums'
                    :label='album.title'
                    :value='album.id'
                    @click.native='preventExpansion'
                  )
              v-expansion-panel-content
                v-skeleton-loader(type='image' :loading='!album.mediaItems')
                  v-row
                    v-col.pa-1(
                      cols='3' sm='2' md='1'
                      v-for='mediaItem in album.mediaItems'
                      :key='mediaItem.id'
                    )
                      v-card(@click='setWallpaper(mediaItem)')
                        v-img(
                          :src='mediaItem | medUrl'
                          :lazy-src='mediaItem | smallUrl'
                          aspect-ratio='1'
                        )
</template>

<script>
import Vue from 'vue';

export default {
  name: 'App',
  mounted() {
    window.eel.get_favorites()(({ mediaItems }) => {
      this.$data.favorites = mediaItems;
    })

    window.eel.get_albums()(({ albums }) => {
      this.$data.albums= albums;
    })
  },
  filters: {
    smallUrl: function(mediaItem) {
      return mediaItem.baseUrl + `=w25-h25`;
    },
    medUrl: function(mediaItem) {
      return mediaItem.baseUrl + `=w150-h150`;
    },
    largeUrl: function(mediaItem) {
      const MAX_SIZE = 16383;
      const width = mediaItem.mediaMetadata ? mediaItem.mediaMetadata.width : MAX_SIZE;
      const height = mediaItem.mediaMetadata ? mediaItem.mediaMetadata.height : MAX_SIZE;
      return mediaItem.baseUrl + `=w${width}-h${height}`;
    }
  },
  methods: {
    preventExpansion: function(event) {
      event.cancelBubble = true;
    },
    loadAlbum: function(albumId) {

      // TODO: Load next page of photos on scroll

      window.eel.get_album_media_items(albumId)(({ mediaItems }) => {
        this.albums = this.albums.map(album => {
          if (album.id == albumId) {
            album.mediaItems = mediaItems;
          }
          return album;
        })
      })
    },
    setWallpaper: function(mediaItem) {
      Vue.set(mediaItem, 'loading', true);

      let url = this.$options.filters.largeUrl(mediaItem);
      
      window.eel.set_wallpaper(url)(() => {
        Vue.set(mediaItem, 'loading', false);
        this.currentWallpaper = mediaItem;
      })
    }
  },
  data: () => ({
    selectedAlbums: [],
    breadcrumbs: [
      { text: 'Favorites' },
      { text: 'DSC_1028.jpg' }
    ],
    currentWallpaper: {},
    favorites: [],
    albums: []
  }),
};
</script>

<style>
  .current-wallpaper {
    border-radius: 12px !important;
  }

  .albums .v-input__slot {
    /* width: fit-content !important; */
  }
</style>