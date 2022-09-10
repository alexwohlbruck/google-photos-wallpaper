<template lang="pug">
v-skeleton-loader(
  type='list-item@10'
  :loading='!(favorites.length && albums.length)'
)
  h3.subtitle-1.font-weight-bold.ma-3 Selected albums

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
            @click.native='preventExpansion'
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
  },
	methods: {
		preventExpansion: function(event) {
			// Disable click bubbling below an element on click event
			event.cancelBubble = true;
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