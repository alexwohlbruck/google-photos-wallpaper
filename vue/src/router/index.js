import Vue from 'vue'
import Router from 'vue-router'

Vue.use(Router)

const routes = [
  {
    path: '/',
    name: 'home',
    component: () => import('@/pages/Home.vue'),
    children: [
      {
        path: 'albums',
        name: 'albums',
        component: () => import('@/pages/Albums.vue')
      },
      {
        path: 'album/:albumId',
        name: 'album',
        component: () => import('@/pages/Album.vue')
      }
    ]
  }
]

const router = new Router({
  mode: 'history',
  routes
})

export default router
