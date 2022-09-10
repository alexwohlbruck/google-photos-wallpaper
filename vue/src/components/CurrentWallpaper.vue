<template lang="pug">
.current-wallpaper.mb-12
  //- Current item details
  v-skeleton-loader(type='sentences, heading' v-if='!options.currentWallpaper.source')

  .d-flex.flex-column.gap-1(v-if='options.currentWallpaper.source')
    
    div
      h3.headline.font-weight-black Current wallpaper
      
      v-breadcrumbs.path.px-1.py-0(:items='breadcrumbs')
        template(v-slot:divider)
          v-icon mdi-chevron-right
      
    preview(
      :image-url='options.currentWallpaper.baseUrl'
      :aspect-ratio='displayAspectRatio'
    )

    .d-flex.gap-half
      v-btn.flex-1(
        outlined
        @click='setWallpaperByDirection("prev")'
        :loading='loadingPrev'
      )
        v-icon.mr-1(left) mdi-arrow-left
        span(v-if='$vuetify.breakpoint.smAndUp') Previous

      v-btn.flex-1(
        outlined
        @click='setWallpaperRandom()'
        :loading='loadingRandom'
      )
        span(v-if='$vuetify.breakpoint.smAndUp') Random
        v-icon.mr-1(right) mdi-shuffle

      v-btn.flex-1(
        outlined
        @click='setWallpaperByDirection("next")'
        :loading='loadingNext'
      )
        span(v-if='$vuetify.breakpoint.smAndUp') Next
        v-icon.mr-1(right) mdi-arrow-right

    .d-flex.flex-column
      h4.text-h4.mb-2 Update every
      .d-flex.align-center.gap-half
        v-text-field.shrink(
          outlined
          dense
          hide-details
          type='number'
          v-model='options.schedule.interval'
          style='width: 80px'
          @change='setSchedule'
          min='1'
          step='1'
        )
        v-select.shrink(
          outlined
          dense
          hide-details
          :items='intervalUnits'
          :item-text='intervalDisplay'
          v-model='options.schedule.unit'
          style='width: 120px'
          @change='setSchedule'
        )
</template>

<script>
import { mapState } from 'vuex';
import Preview from '@/components/Preview.vue';

export default {
  name: 'CurrentWallpaper',
  components: {
    Preview,
  },
  data: () => ({
		loadingNext: false,
		loadingPrev: false,
		loadingRandom: false,
    intervalUnits: [
      {
        value: "minutes",
        text: "minutes",
      },
      {
        value: "hours",
        text: "hours",
      },
      {
        value: "days",
        text: "days",
      },
      {
        value: "weeks",
        text: "weeks",
      },
    ],
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
    displayAspectRatio: function() {
      return screen.width / screen.height;
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
    intervalDisplay: function(unit) {
      return this.options.schedule.interval == 1 ? unit.text.slice(0, -1) : unit.text;
    },
  }
}
</script>

<style lang="scss">
.current-wallpaper {
  overflow: visible !important;
}

.path {
  .v-breadcrumbs__divider {
    padding: 0 .3em !important;
  }
}
</style>