<template lang="pug">
	v-app(:style='`background-image: url(${wallpaperUrl})`')
		.blur
			v-overlay(:value='appLoading')
				.d-flex.flex-column.align-center
					v-progress-circular(indeterminate)
					p.pa-2 {{ appLoading }}

			v-content
				router-view
					
</template>

<script>

export default {
	name: 'App',
	computed: {
		wallpaperUrl() {
			return this.$store.state.options?.currentWallpaper?.baseUrl;
		},
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
	#app {
		background-size: cover;
		background-position: center;
		background-repeat: no-repeat;
		background-attachment: fixed;
	}

	.round {
		border-radius: 10px !important;
	}

	@media screen and (min-width: 960px) {
		.current-wallpaper {
			position: sticky;
			top: 32px;
			align-self: flex-start;
			overflow-y: auto;
		}
	}

	.blur {
		backdrop-filter: blur(30px) saturate(4) contrast(.4) brightness(.9);
		min-height: 100vh;
	}
</style>