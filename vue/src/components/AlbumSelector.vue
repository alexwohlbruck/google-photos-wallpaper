<template lang="pug">
v-skeleton-loader(
  type='list-item@10'
  :loading='!(favorites.length && albums.length)'
)
  h3.subtitle-1.font-weight-bold.ma-3 Selected albums

  v-container
    v-row(dense)
      v-col(cols='6' sm='4' xl='3' v-for='album in albumsIncludingFavorites' :key='album.id')
        v-card.album(
          :to='{ name: "album", params: { id: album.id } }'
          :href='album.url'
          target='_blank'
          rel='noopener'
          :color='album.color'
        )
          v-img(
            :src='album.coverPhotoBaseUrl'
            :alt='album.title'
            :aspect-ratio='1'
            @click='openAlbum(album)'
          )
            v-checkbox.select(
              color='success'
              class='v-card__actions'
              v-model='options.selectedAlbums'
              :value='album.id'
              @change='setSelectedAlbums'
              @click.native='cancelBubble'
              on-icon='mdi-checkbox-marked-circle'
              off-icon='mdi-checkbox-blank-circle-outline'
            )
              v-icon {{ album.selected ? 'mdi-check' : 'mdi-plus' }}

            v-icon.icon(v-if='album.icon') {{ album.icon }}
            
            .scrim(:class='{ light: album.icon }')
              v-card-title.body-2 {{ album.title }}
              v-card-subtitle.caption {{ album.mediaItemsCount }} items

  v-expansion-panels.albums(accordion multiple light)
    //- Favorites panel
    v-expansion-panel
      v-expansion-panel-header.py-0(v-slot='open')
        v-layout
          v-checkbox(
            v-model='options.selectedAlbums'
            label='Favorites'
            value='FAVORITES'
            @change='setSelectedAlbums'
            @click.native='preventExpansion'
          )
      v-expansion-panel-content
        media-items(:media-items='favorites' :source='{id: "FAVORITES", title: "Favorites"}')
        

    //- Album panels list           
    v-expansion-panel(
      v-for='album in albums'
      :key='album.id'
      @change='loadAlbum(album.id)'
    )
      v-expansion-panel-header.py-0(v-slot='open')
        v-layout
          v-checkbox(
            v-model='options.selectedAlbums'
            :label='album.title'
            :value='album.id'
            @change='setSelectedAlbums'
            @click.native='cancelBubble'
          )
      v-expansion-panel-content
        v-skeleton-loader(type='image' :loading='!album.mediaItems')
          media-items(
            :media-items='album.mediaItems'
            :source='album'
            :disabled='!options.selectedAlbums.find(a => a == album.id)'
          )
</template>

<script>
import { mapState } from 'vuex';
import MediaItems from '@/components/MediaItems.vue';

export default {
  name: 'AlbumSelector',
  components: {
    MediaItems
  },
  computed: {
		...mapState([
			'options',
			'favorites',
			'albums'
		]),
    albumsIncludingFavorites() {
      return [
        {
          id: 'FAVORITES',
          title: 'Favorites',
          mediaItemsCount: this.favorites.length,
          mediaItems: this.favorites,
          icon: 'mdi-heart',
          color: 'pink',
        },
        ...this.albums
      ]
    }
  },
	methods: {
		cancelBubble: function(event) {
			// Disable click bubbling below an element on click event
			event.cancelBubble = true;
		},
    openAlbum: function(album) {
      // Open album in new tab
      alert('clicked');
      console.log(album);
    },
    selectAlbum: function(album) {
      // Select album
      alert('selected');
      console.log(album);
    },
		loadAlbum: function(albumId) {
			// TODO: Load next page of photos on scroll
			this.$store.dispatch('getAlbum', { albumId });
		}, 
		setSelectedAlbums: function() {
			this.$data.appLoading = 'Updating album data';
			window.eel.set_selected_albums(this.$store.state.options.selectedAlbums)(() => {
				this.$data.appLoading = '';
			});
		},
  }
}
</script>

<style lang="scss">
.album {
  .select {
    position: absolute;
    margin: .2em;
  }

  .icon {
    position: absolute;
    top: 50%;
    left: 50%;
    transform: translate(-50%, -50%);
    font-size: 2em;
  }

  .scrim {
    width: 100%;
    height: 100%;
    display: flex;
    flex-direction: column;
    justify-content: flex-end;

    * {
      overflow: hidden;
      white-space: nowrap;
      text-overflow: ellipsis;
      display: block;
    }

    &::before {
      content: '';
      position: absolute;
      z-index: -1;
      top: 0;
      left: 0;
      width: 100%;
      height: 100%;
      background: linear-gradient(0deg, rgba(0,0,0,0.7) 0%, rgba(0,0,0,0) 45%, rgba(0,0,0,0) 55%, rgba(0,0,0,.7) 100%);
    }

    &.light {
      &::before {
        background: linear-gradient(0deg, rgba(0,0,0,0.2) 0%, rgba(0,0,0,0) 45%, rgba(0,0,0,0) 55%, rgba(0,0,0,.3) 100%);
      }
    }
  }
}
</style>