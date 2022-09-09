<template lang="pug">
	v-app
		v-overlay(:value='appLoading')
			.d-flex.flex-column.align-center
				v-progress-circular(indeterminate)
				p.pa-2 {{ appLoading }}

		v-content.pt-4
			v-container
				preview
				album-selector
</template>

<script>
import Preview from '@/components/Preview.vue';
import AlbumSelector from '@/components/AlbumSelector.vue';

export default {
	name: 'App',
	components: {
		Preview,
		AlbumSelector,
	},
	async beforeCreate() {
		this.$store.dispatch('getFavorites');
		this.$store.dispatch('getAlbums');
		await this.$store.dispatch('getUserOptions');
	},

	mounted() {
		var that = this;
		console.log(window.eel);
		let onWallChanged = mediaItem => {
			that.$store.commit('setCurrentWallpaper', { mediaItem })
		}
		window.eel.expose(onWallChanged, 'wallpaper_changed');
	},

	data: () => ({
		appLoading: '', // Display an overlay over entire app with loading message
	}),
};
</script>

<style>
	.round {
		border-radius: 10px !important;
	}
</style>