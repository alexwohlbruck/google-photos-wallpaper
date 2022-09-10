<template lang="pug">
.album

  .d-flex.align-center  
    v-btn(icon)
      v-icon mdi-chevron-left
    h3.subtitle-1.font-weight-bold {{ album.title }}

  pre(style='max-width: 500px') {{album}}
  //- media-items(
  //-   :media-items='album.mediaItems'
  //-   :source='album'
  //-   :disabled='!options.selectedAlbums.find(a => a == album.id)'
  //- )
</template>

<script>
import { mapState } from 'vuex'
import MediaItems from '@/components/MediaItems.vue';

export default {
  name: 'Album',
  components: {
    MediaItems
  },
  computed: {
    album() {
      return this.$store.state.albums.find(a => a.id == this.$route.params.albumId);
    },
    ...mapState([
      'options',
    ]),
  },
}
</script>