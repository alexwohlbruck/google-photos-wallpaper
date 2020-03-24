<template lang="pug">
	v-app
		v-app-bar(
			app
			max-height='1000'
			flat
		)
			v-app-bar-nav-icon

		v-overlay(:value='appLoading')
			.d-flex.flex-column.align-center
				v-progress-circular(indeterminate)
				p.pa-2 {{ appLoading }}

		v-content
			v-container
				//- Current wallpaper preview
				v-row.mb-12.align-center
					v-col(cols='5')
						v-card.round(elevation='22')
							v-skeleton-loader(type='image' max-height='350' v-if='options.currentWallpaper.baseUrl')
							v-img.round(v-if='!options.currentWallpaper.baseUrl' :src='options.currentWallpaper.baseUrl' max-height='350')

					//- Current item details
					v-col.pb-10(cols='7')
						.pa-5
							v-skeleton-loader(type='sentences, heading' v-if='!options.currentWallpaper')
							div(v-if='options.currentWallpaper')
								h3.headline.font-weight-bold Current wallpaper
								v-breadcrumbs.px-1.pt-1.pb-2(:items='breadcrumbs')
									template(v-slot:divider)
										v-icon mdi-arrow-right
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
										v-model='options.selectedAlbums'
										label='Favorites'
										value='FAVORITES'
										@change='setSelectedAlbums'
										@click.native='preventExpansion'
									)
							v-expansion-panel-content
								media-items(:media-items='favorites' source='Favorites')
								

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
									media-items(:media-items='album.mediaItems' :source='album.title')
</template>

<script>
import { mapState } from 'vuex';
import MediaItems from '@/components/MediaItems.vue';

export default {
	name: 'App',

	components: {
		MediaItems
	},

	async beforeCreate() {
		this.$store.dispatch('getFavorites');
		this.$store.dispatch('getAlbums');
		await this.$store.dispatch('getUserOptions');
	},

	data: () => ({
		appLoading: '' // Display an overlay over entire app with loading message
	}),

	computed: {
		breadcrumbs: function() {
			return [
				{ text: this.options.currentWallpaper.source },
				{ text: this.options.currentWallpaper.filename }
			]
		},
		...mapState([
			'options',
			'favorites',
			'albums'	
		])
	},

	methods: {
		preventExpansion: function(event) {
			// Disable click bubbling below an element on click event
			event.cancelBubble = true;
		},
		loadAlbum: function(albumId) {
			// TODO: Load next page of photos on scroll
			this.$store.dispatch('getAlbum', { albumId })
		}, 
		setSelectedAlbums: function() {
			this.$data.appLoading = 'Updating album data';
			window.eel.set_selected_albums(this.$store.state.options.selectedAlbums)(() => {
				this.$data.appLoading = '';
			});
		},
	}
};
</script>

<style>
	.round {
		border-radius: 10px !important;
	}
</style>