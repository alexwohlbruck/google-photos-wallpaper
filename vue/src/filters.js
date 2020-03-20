import Vue from 'vue'

Vue.filter('smallUrl', function(mediaItem) {
    return mediaItem.baseUrl + `=w25-h25`;
});

Vue.filter('medUrl', function(mediaItem) {
    return mediaItem.baseUrl + `=w150-h150`;
});