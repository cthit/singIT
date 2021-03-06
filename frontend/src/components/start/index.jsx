//@flow

import React, {PropTypes} from 'react';
import ReactDOM from 'react-dom';
import Infinite from 'react-infinite';
import Fuse from 'fuse.js';
import _ from 'lodash';
import {fetchJson} from '../../services/backend';
import SongItem from '../song-item'

import styles from './style.css'


const timeSort = (a, b) => {
  return new Date(b.created_at) - new Date(a.created_at);
}

const artistSort = (a, b) => {
  const artistDiff = a.artist.toLowerCase().localeCompare(b.artist.toLowerCase());
  return artistDiff == 0
    ? a.title.toLowerCase().localeCompare(b.title.toLowerCase())		
    : artistDiff;
}

const Start = React.createClass({
    getInitialState() {
      return {
        searchString: '',
        songs: [],
        filteredSongs: [],
        sortingMethod: artistSort
      };
    },

    componentWillMount() {
      this.debouncedPerformSearch = _.debounce(this.performSearch, 100);
      fetchJson('/api/songs.json').then(songs => {
        const sortedSongs = songs
          .filter(s => s.title && s.title.length > 0 && s.artist && s.artist.length > 0)
          .sort(artistSort);
        const options = {
            shouldSort: true,
            maxPatternLength: 32,
            keys: [
                  "title",
                  "artist"
              ]
        };
        this.fuse = new Fuse(sortedSongs, options);
        this.setState({
          songs: sortedSongs,
          filteredSongs: sortedSongs,
          sortingMethod: artistSort
        })
      });
    },

    handleSearchInput(event) {
      ReactDOM.findDOMNode(this.songlist).scrollTop = 0

      this.setState({
        searchString: event.target.value
      }, this.debouncedPerformSearch)
    },

    performSearch() {
      const { searchString, songs } = this.state;

      if (searchString.length <= 1) {
        this.setState({ filteredSongs: songs });
        return;
      }
      const result = this.fuse.search(searchString);

      this.setState({
        filteredSongs: result
      })
    },

    sortSongs(sortingMethod) {
      const songs = this.state.songs.sort(sortingMethod);
      const filteredSongs = this.state.filteredSongs.sort(sortingMethod);
      this.setState({songs, filteredSongs, sortingMethod})
    },

    renderSortButton() {
      switch (this.state.sortingMethod) {
        case artistSort:
          return <i className="fa fa-clock-o" onClick={() => this.sortSongs(timeSort)}></i>
        case timeSort:
          return <i className="fa fa-sort-alpha-asc" onClick={() => this.sortSongs(artistSort)}></i>
        default:
          return <i className="fa fa-clock-o" onClick={() => this.sortSongs(timeSort)}></i>
        }
    },

    render() {
        const { filteredSongs, searchString } = this.state;

        return (
          <div className={styles.container}>
            <div className={styles.sticky}>
              <div className={styles.searchAndSort}>
                <input type="text"
                       autoFocus={true}
                       className={styles.searchBox}
                       ref="searchInput"
                       onChange={this.handleSearchInput}
                       value={searchString}
                       placeholder="Search" />
                 {this.renderSortButton()}
               </div>
              <div className={styles.hits}>Hits: {filteredSongs.length}</div>
            </div>
            <Infinite containerHeight={window.innerHeight}
                      ref={(songlist) => this.songlist = songlist}
                      elementHeight={48}
                      preloadAdditionalHeight={window.innerHeight*2}
                      className={styles.songList}>
              {filteredSongs.map(song => (
                <SongItem key={song.song_hash} song={song} />
              ))}
            </Infinite>
          </div>
        )
    }
});

export default Start
