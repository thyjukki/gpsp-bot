def app = null
def youtube_dlp_version = null
pipeline {
  agent { label 'linux&&docker' }
  options {
    disableConcurrentBuilds()
  }
  triggers {
    cron('H 2 * * *')
  }
  stages {
    stage("Get package version") {
      agent {
        docker {
          image 'python:3.11-slim'
          label 'linux&&docker'
        }
      }
      steps {
        script {
          youtube_dlp_version = sh(script: "pip index versions yt-dlp | grep yt-dlp | head -n 1 | awk '{print \$2}' | tr -d '()'", returnStdout: true).trim()
        }
      }
    }
    stage("Build") {
      steps {
        script {
          sh "mv .containerignore .dockerignore"
          app = docker.build("jukki/kld-bot", "--build-arg YOUTUBE_DLP_VERSION=${youtube_dlp_version} -f ./Containerfile .")
        }
      }
    }
    stage("Docker push") {
      when {
        branch 'main'
      }
      steps {
        script {
          docker.withRegistry('https://nexus.jukk.it', 'nexus-jenkins-user' ) {
            app.push("0.${BUILD_NUMBER}")
            app.push("latest")
          }
        }
      }
    }
    stage('Deploy App') {
      when {
        branch 'main'
      }
      agent {
        docker {
          image 'caprover/cli-caprover'
          label 'linux&&docker'
        }
      }
      steps {
        withCredentials([string(credentialsId: 'caprover-password', variable: 'CAPROVER_PASSWORD')]) {
          sh "caprover deploy -c captain-definition"
        }
      }
    }
  }
}