//
//  ReceiveView.swift
//  BitBridge
//
//  Created by Dang Minh Tien on 19/2/25.
//

import SwiftUI
import Foundation
import SharedTypes

struct ReceiveView: View {
    @Environment(\.safeAreaInsets) private var safeAreaInsets
    @Environment(\.screenSize) private var screenSize
    @EnvironmentObject private var core: Core
    
    @State var receiveSessions: [ReceiveSessionViewModel] = []
    @State var isShowingOptionAll = false
    
    var body: some View {
        ZStack {
            StunningBackgroundGradient()
            ScrollView {
                LazyVStack(spacing: 8) {
                    LogoScene(gltfFileName: "Rocket", logoScale: 1.5)
                        .frame(width: screenSize.width, height: 100)
                        .overlay(Theme.gradientHeading
                            .opacity(0.5)
                            .blur(radius: 15)
                            .frame(width: .infinity, height: screenSize.width / 2)
                        )
                    
                    Text("Your Inbox")
                        .padding(.horizontal, 20)
                        .multilineTextAlignment(.center)
                        .modifier(Heading2())
                    
                    Spacer().frame(height: 10)
                    UpgradePremiumButton()
                    
                    HStack(alignment: .center) {
                        Spacer()
                        Button(action: { isShowingOptionAll = true }) {
                            ImageAsset.More.image
                                .foregroundColor(Theme.PrimaryText.color)
                                .scaleEffect(1.6)
                                .confirmationDialog(
                                    "Options", isPresented: $isShowingOptionAll) {
                                        Button("Remove") {
                                            Task {
                                                
                                            }
                                        }
                                        Button("Open") {
                                        }
                                    }
                        }
                        .frame(minWidth: 35, alignment: .trailing)
                    }
                    .padding(.all, 5)
                    
                    VStack {
                        ForEach(self.receiveSessions, id: \.self.id) { item in
                            ReceiveSessionView(session: item)
                        }
                    }
                }
            }
        }
        .onReceive(self.core.transfer, perform: { value in
            let receivedSessions = value?.received_sessions ?? [];
            
            if receivedSessions.count != self.receiveSessions.count {
                self.receiveSessions = receivedSessions
            }
        })
    }
}

#Preview {
    ReceiveView()
        .environmentObject(CoreMock.withSelectedFileTransfers() as Core)

}
