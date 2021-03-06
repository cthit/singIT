class SongsController < ApplicationController
  before_action :set_song, only: [:show, :edit, :update, :destroy]
  before_action :restrict_access, only: [:batch_create, :create, :update, :destroy]

  # GET /songs
  # GET /songs.json
  def index
    @songs = Song.all

    respond_to do |format|
      format.html
      format.json do
        @json = Rails.cache.fetch @songs do
          @songs.to_json
        end
        render json: @json
      end
    end
  end

  # GET /songs/1
  # GET /songs/1.json
  def show
  end

  # GET /songs/new
  def new
    @song = Song.new
  end

  # GET /songs/1/edit
  def edit
  end

  # POST /songs
  # POST /songs.json
  def create
    @song = Song.new(song_params)

    respond_to do |format|
      if @song.save
        format.html { redirect_to @song, notice: 'Song was successfully created.' }
        format.json { render :show, status: :created, location: @song }
      else
        format.html { render :new }
        format.json { render json: @song.errors, status: :unprocessable_entity }
      end
    end
  end

  def batch_create
    @songs_params = params[:songs].map do |song|
      song.permit(permitted_keys)
    end
    @songs = @songs_params.map do |s|
      song = Song.where(song_hash: s[:song_hash]).first_or_initialize
      song.update_attributes(s.except(:song_hash))
      song
    end

    all_success = @songs.all?(&:persisted?)

    respond_to do |format|
      if all_success
        format.json { render json: @songs, status: :created }
      else
        format.json { render json: @songs.map(&:errors), status: :unprocessable_entity }
      end
    end


  end

  # PATCH/PUT /songs/1
  # PATCH/PUT /songs/1.json
  def update
    respond_to do |format|
      if @song.update(song_params)
        format.html { redirect_to @song, notice: 'Song was successfully updated.' }
        format.json { render :show, status: :ok, location: @song }
      else
        format.html { render :edit }
        format.json { render json: @song.errors, status: :unprocessable_entity }
      end
    end
  end

  # DELETE /songs/1
  # DELETE /songs/1.json
  def destroy
    @song.destroy
    respond_to do |format|
      format.html { redirect_to songs_url, notice: 'Song was successfully destroyed.' }
      format.json { head :no_content }
    end
  end

  private
    # Use callbacks to share common setup or constraints between actions.
    def set_song
      @song = Song.find(params[:id])
    end

    # Never trust parameters from the scary internet, only allow the white list through.
    def song_params
      params.require(:song).permit(permitted_keys)
    end

    def permitted_keys
      [:title, :artist, :cover, :song_hash, :genre]
    end

    def restrict_access
      authenticate_or_request_with_http_token do |token, options|
        ApiKey.exists?(access_token: token)
      end
    end

end
