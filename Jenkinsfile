def app = null
pipeline {
  agent { label 'linux&&docker' }
  options {
    disableConcurrentBuilds()
  }
  triggers {
    cron('H 4/* 0 0 1-5')
  }
  stages {
    /*stage("SonarQube Analysis") {
      agent {
        docker {
          label 'linux&&docker'
          image 'sonarsource/sonar-scanner-cli:5.0.1'
        }
      }
      steps {
        withSonarQubeEnv('SonarQube Jukki') {
          sh "sonar-scanner"
        }
      }
    }
    stage("Quality Gate") {
        steps {
          timeout(time: 1, unit: 'HOURS') {
            waitForQualityGate abortPipeline: true
          }
        }
    }*/
    stage("Build") {
      steps {
        script {
          app = docker.build("jukki/kld-bot")
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