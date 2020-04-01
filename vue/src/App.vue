<template lang="pug">
	v-app
		v-overlay(:value='appLoading')
			.d-flex.flex-column.align-center
				v-progress-circular(indeterminate)
				p.pa-2 {{ appLoading }}

		v-content.pt-4
			v-container
				//- Current wallpaper preview
				v-row.mb-12.align-center
					v-col(cols='5')
						v-card.round(elevation='22')
							v-skeleton-loader(type='image' v-if='!options.currentWallpaper.baseUrl')
							v-img.round(v-if='options.currentWallpaper.baseUrl' :src='options.currentWallpaper.baseUrl' max-height='350')

					//- Current item details
					v-col.pb-5(cols='7')
						.pa-5
							v-skeleton-loader(type='sentences, heading' v-if='!options.currentWallpaper.source')
							div(v-if='options.currentWallpaper.source')
								h3.headline.font-weight-bold Current wallpaper
								
								v-breadcrumbs.px-1.pt-1.pb-2(:items='breadcrumbs')
									template(v-slot:divider)
										v-icon mdi-arrow-right

								div
									v-btn.ma-1(
										outlined
										@click='setWallpaperByDirection("prev")'
										:loading='loadingPrev'
									) Previous

									v-btn.ma-1(
										outlined
										@click='setWallpaperByDirection("next")'
										:loading='loadingNext'
									) Next

									v-btn.ma-1(
										outlined
										@click='setWallpaperRandom()'
										:loading='loadingRandom'
									) Random

								.d-flex.align-center.my-3
									span.mr-1 Update every
									v-text-field.shrink.mx-1.hide-arrows(
										outlined
										dense
										hide-details
										type='number'
										v-model='update.interval'
										style='width: 65px'
									)
									v-select.shrink.mx-1(
										outlined
										dense
										hide-details
										:items='["minutes", "hours", "days", "weeks"]'
										v-model='update.unit'
										style='width: 120px'
									)
				
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
		appLoading: '', // Display an overlay over entire app with loading message
		loadingNext: false,
		loadingPrev: false,
		loadingRandom: false,
		update: {
			interval: 5,
			unit: 'days'
		}
	}),

	computed: {
		breadcrumbs: function() {
			return [
				{ text: this.options.currentWallpaper.source.title },
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
			this.$store.dispatch('getAlbum', { albumId });
		}, 
		setSelectedAlbums: function() {
			this.$data.appLoading = 'Updating album data';
			window.eel.set_selected_albums(this.$store.state.options.selectedAlbums)(() => {
				this.$data.appLoading = '';
			});
		},
		setWallpaperByDirection: async function(direction) {
			let directionCamel = direction.charAt(0).toUpperCase() + direction.slice(1)
			this.$data[`loading${directionCamel}`] = true;
			await this.$store.dispatch('setWallpaperByDirection', { direction });
			this.$data[`loading${directionCamel}`] = false;
		},
		setWallpaperRandom: async function() {
			this.$data.loadingRandom = true;
			await this.$store.dispatch('setWallpaperRandom');
			this.$data.loadingRandom = false;
		}
	}
};
</script>

<style>
	.round {
		border-radius: 10px !important;
	}
</style>