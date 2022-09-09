<template lang="pug">
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
            v-model='options.schedule.interval'
            style='width: 65px'
            @change='setSchedule'
          )
          v-select.shrink.mx-1(
            outlined
            dense
            hide-details
            :items='["minutes", "hours", "days", "weeks"]'
            v-model='options.schedule.unit'
            style='width: 120px'
            @change='setSchedule'
          )
</template>

<script>
import { mapState } from 'vuex';

export default {
  name: 'Preview',
  data: () => ({
		loadingNext: false,
		loadingPrev: false,
		loadingRandom: false
  }),
	computed: {
		...mapState([
			'options'
		]),
		breadcrumbs: function() {
			return [
				{ text: this.options.currentWallpaper.source.title },
				{ text: this.options.currentWallpaper.filename }
			]
		},
	},
  methods: {
		setSchedule: function() {
			let interval = parseInt(this.$store.state.options.schedule.interval),
				unit = this.$store.state.options.schedule.unit;

			// Only update if there is a value inputted
			if (interval) {
				this.$store.dispatch('setSchedule', { interval, unit });
			}
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
		},
  }
}
</script>
