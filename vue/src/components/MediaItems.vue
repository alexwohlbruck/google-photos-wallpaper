<template lang="pug">
    //- A grid of media item thumbnails that can be selected to set the wallpaper
    v-row
        v-col.pa-1(
            cols='3' sm='2' md='1'
            v-for='mediaItem in mediaItems'
            :key='mediaItem.id'
        )
            v-card(@click='setWallpaper(mediaItem, source);')
                v-img(
                    :src='mediaItem | medUrl'
                    :lazy-src='mediaItem | smallUrl'
                    aspect-ratio='1'
                )
                    //- Loading overlay
                    v-overlay(absolute :value='mediaItem.loading == true')
                        v-progress-circular(indeterminate)
                    //- Current set wallpaper overlay
                    v-overlay(
                        absolute color='blue'
                        v-if='currentWallpaper'
                        :value='mediaItem.id == currentWallpaper.id'
                    )
                        v-icon mdi-check
</template>

<script>
import Vue from 'vue';

export default {
    name: 'mediaItems',
    props: ['mediaItems', 'source'],
    methods: {
        setWallpaper: function(mediaItem, source) {
			Vue.set(mediaItem, 'loading', true);

			// Attach name of source for reference in the current wallpaper preview
			mediaItem.source = source;

			window.eel.set_wallpaper(mediaItem)(() => {
                Vue.set(mediaItem, 'loading', false);

                // TODO: Use Vuex for central options data store
				// this.currentWallpaper = mediaItem;
			});
		}
    }
}
</script>