<template lang="pug">
.preview
  v-skeleton-loader(type='image' v-if='!imageUrl')
  v-img.round.elevation-22(
    v-else
    :src='imageUrl'
    :aspect-ratio='aspectRatio'
  )
    .menu-bar
      .menu-left
        v-icon.apple-logo(color='white') mdi-apple
        .menu-item(v-for='i in menuItemsLeft' :style='{width: `${i}%`}')
      v-spacer
      .menu-right.display-flex.justify-end
        .menu-item(v-for='i in 6')
        .menu-item.time
    
    .dock
      .dock-item(v-for='i in dockItems' :style='{background: i}')
</template>

<script>
export default {
  name: 'Preview',
  props: ['imageUrl', 'aspectRatio'],
  computed: {
    menuItemsLeft() {
      return [...Array(6).keys()].map(() => {
        return Math.ceil(Math.random() * 7) + 7;
      })
    },
    dockItems() {
      return [...Array(15).keys()].map(() => {
        return `hsl(${Math.random() * 360}, ${(Math.random() * 30) + 70}%, ${(Math.random() * 50) + 20}%)`;
      })
    }
  }
}
</script>

<style lang="scss">
  .frosted-glass {
    backdrop-filter: blur(20px) saturate(2) brightness(.9);
  }
  .menu-bar {
    @extend .frosted-glass;

    position: absolute;
    top: 0;
    left: 0;
    width: 100%;
    height: 4%;
    display: flex;

    .apple-logo {
      margin-left: 4%;
      margin-right: 1%;
      font-size: 1.5vw !important;

      @media (min-width: 960px) {
        font-size: .6rem !important;
      }
    }

    .menu-item {
      width: 5px;
      height: 20%;
      margin: 0 2%;
      background-color: rgba(255, 255, 255, .5);
      border-radius: 5px;

      &.time {
        width: 15%;
      }
    }

    .menu-left, .menu-right {
      display: flex;
      position: relative;
      height: 100%;
      width: 40%;
      align-items: center;
    }
  }

  .dock {
    @extend .frosted-glass;

    position: absolute;
    display: flex;
    align-items: center;
    justify-content: center;
    height: 8%;
    width: 66%;
    bottom: 0;
    left: 50%;
    transform: translateX(-50%);
    border-radius: 10px;

    .dock-item {
      height: 70%;
      aspect-ratio: 1;
      margin: .5%;
      border-radius: 5px;
    }
  }

</style>