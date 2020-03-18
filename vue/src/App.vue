<template lang="pug">
  v-app
    v-app-bar(
      app
      max-height='1000'
      flat
    )
      v-app-bar-nav-icon
      
      v-spacer
      
      v-btn(icon)
        v-icon mdi-save

        //- p Favorites > DSC_0752.jpg

    v-content
      v-container
        //- Current wallpaper preview
        v-row.mb-12.align-center
          v-col(cols='5')
            v-card.current-wallpaper(elevation='22')
              v-img(src='https://picsum.photos/1080/400?random' max-height='400')

          v-col.pb-10(cols='7')
            .pa-5
              h3.headline.font-weight-bold Current wallpaper
              v-breadcrumbs.px-1.pt-1.pb-2(:items='breadcrumbs')
                template(v-slot:divider)
                  v-icon mdi-chevron-right
              v-btn.ma-1(outlined) Previous
              v-btn.ma-1(outlined) Next
              v-btn.ma-1(outlined) Random
        
        //- Album selection
        h3.subtitle-1.font-weight-bold.ma-3 Selected albums
        v-expansion-panels(accordion)
          each panel in [1,2,3,4]
            v-expansion-panel
              v-expansion-panel-header.py-0(v-slot='open')
                v-checkbox(v-model='selectedAlbums' value='favorites' label='Favorites')
              v-expansion-panel-content
                v-row
                  each pic in [1,2,3,4,5,6,7,8,9,10,11,12]
                    v-col.pa-1(cols='3' sm='2' md='1')
                      v-card
                        v-img(src='https://picsum.photos/200/200?random' aspect-ratio='1')
</template>

<script>
export default {
  name: 'App',
  data: () => ({
    breadcrumbs: [
      { text: 'Favorites' },
      { text: 'DSC_1028.jpg' }
    ]
  }),
};
</script>

<style>
  .current-wallpaper {
    border-radius: 12px !important;
  }
</style>