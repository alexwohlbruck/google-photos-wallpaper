<template lang="pug">
v-container.d-flex.flex-column.flex-md-row.gap-2
  v-row.mt-4
    v-col
      current-wallpaper
    v-col
      router-view
</template>

<script>
import CurrentWallpaper from '@/components/CurrentWallpaper.vue';
// import AlbumSelector from '@/components/AlbumSelector.vue';

export default {
  name: 'Home',
	components: {
		CurrentWallpaper,
		// AlbumSelector,
	},
}
</script>